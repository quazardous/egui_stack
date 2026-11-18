//! The ghost of a dragged tab, or of a whole tab group, under the pointer: one drawing for
//! both, a single tab's ghost being a one-tab group's.

use std::sync::Arc;

use egui::{
    Galley, LayerId, Order, Pos2, Rect, TextStyle, Ui, UiBuilder, Vec2, WidgetText, pos2, vec2,
};

use crate::{
    DockArea, DraggedGroupMore, DraggedGroupPaint, DraggedGroupTab, NodePath, PartState, TabIndex,
    TabMeasure, TabViewer, widgets::painter::GHOST_OPACITY,
};

/// The smallest and largest size of a group's ghost, in points: it takes its leaf's size within
/// them.
const GHOST_MIN: Vec2 = vec2(160.0, 80.0);
const GHOST_MAX: Vec2 = vec2(320.0, 200.0);

/// Where the pointer is in the ghost: near the left end of its tab bar.
const GRAB_X: f32 = 16.0;

/// The room left around the "+N" of the tabs that do not fit.
const MORE_PADDING: f32 = 12.0;

/// Which of the tabs of `widths` fit in `room` with `spacing` between them: the first ones,
/// the `active` one always among them, in order.
pub(super) fn fitting_tabs(widths: &[f32], spacing: f32, room: f32, active: usize) -> Vec<usize> {
    let width_of = |tabs: &[usize]| -> f32 {
        tabs.iter().map(|&i| widths[i]).sum::<f32>() + spacing * tabs.len().saturating_sub(1) as f32
    };
    let mut shown: Vec<usize> = Vec::new();
    for i in 0..widths.len() {
        shown.push(i);
        if width_of(&shown) > room {
            shown.pop();
            break;
        }
    }
    if active < widths.len() && !shown.contains(&active) {
        while !shown.is_empty() && width_of(&shown) + spacing + widths[active] > room {
            shown.pop();
        }
        shown.push(active);
    }
    shown
}

impl<Tab> DockArea<'_, Tab> {
    /// Paints the ghost of what is dragged under `pointer`, on a layer above the drop overlay:
    /// the group at `path` through [`DockPainter::dragged_group`](crate::DockPainter::dragged_group),
    /// or, with `tab`, that one tab of it, as a one-tab group, through
    /// [`DockPainter::dragged_tab`](crate::DockPainter::dragged_tab).
    pub(super) fn paint_ghost(
        &mut self,
        ui: &Ui,
        path: NodePath,
        tab: Option<TabIndex>,
        pointer: Pos2,
        tab_viewer: &mut impl TabViewer<Tab = Tab>,
    ) {
        let style = self.style.as_ref().unwrap();
        let leaf_size = self
            .dock_state
            .leaf(path)
            .ok()
            .and_then(|_| self.dock_state[path].rect())
            .map_or(GHOST_MAX, |rect| rect.size());
        let Ok(leaf) = self.dock_state.leaf_mut(path) else {
            return;
        };
        // The tab whose body the ghost shows: the dragged one, or the group's active one.
        let shown_tab = tab.map_or(leaf.active.0, |tab| tab.0);
        let bar_height = style.tab_bar.height;
        let size = {
            let min = vec2(GHOST_MIN.x, GHOST_MIN.y.max(bar_height + 24.0));
            let max = vec2(GHOST_MAX.x, GHOST_MAX.y.max(bar_height + 24.0));
            let default = leaf_size.clamp(min, max);
            match leaf.tabs.get(shown_tab) {
                Some(shown) => tab_viewer.ghost_size(shown, default).clamp(min, max),
                None => default,
            }
        };
        // The tabs the ghost holds, and which of them is shown.
        let (tabs, active) = match tab {
            Some(tab) => match leaf.tabs.get_mut(tab.0) {
                Some(tab) => (vec![tab], 0),
                None => return,
            },
            None => {
                let active = leaf.active.0;
                (leaf.tabs.iter_mut().collect(), active)
            }
        };
        let count = tabs.len();
        if count == 0 {
            return;
        }
        let rect = Rect::from_min_size(pointer - vec2(GRAB_X, bar_height / 2.0), size);
        let bar = Rect::from_min_size(rect.min, vec2(size.x, bar_height));
        let body = Rect::from_min_max(pos2(rect.left(), bar.bottom()), rect.max);

        let layout = |text: WidgetText| -> Arc<Galley> {
            text.into_galley(ui, None, f32::INFINITY, TextStyle::Button)
        };
        let titles: Vec<Arc<Galley>> = tabs
            .into_iter()
            .map(|tab| layout(tab_viewer.title(tab)))
            .collect();
        let widths: Vec<f32> = titles
            .iter()
            .enumerate()
            .map(|(i, title)| {
                self.painter.tab_width(&TabMeasure {
                    title,
                    state: PartState::NONE.with(PartState::SELECTED, i == active),
                    close_width: 0.0,
                    style,
                    tab_style: &style.tab,
                })
            })
            .collect();
        let spacing = style.tab.spacing;
        let all = fitting_tabs(&widths, spacing, size.x, active);
        let (shown, more) = if all.len() == count {
            (all, None)
        } else {
            let label = layout(format!("+{count}").into());
            let room = size.x - label.size().x - MORE_PADDING;
            let shown = fitting_tabs(&widths, spacing, room, active);
            let hidden = count - shown.len();
            let label = layout(format!("+{hidden}").into());
            (shown, Some((hidden, label)))
        };

        let mut x = bar.left();
        let tabs: Vec<DraggedGroupTab> = shown
            .iter()
            .map(|&i| {
                let tab_rect = Rect::from_min_size(pos2(x, bar.top()), vec2(widths[i], bar_height));
                x += widths[i] + spacing;
                DraggedGroupTab {
                    rect: tab_rect,
                    title: Arc::clone(&titles[i]),
                    state: PartState::NONE.with(PartState::SELECTED, i == active),
                }
            })
            .collect();
        let more = more.map(|(hidden, label)| {
            let width = label.size().x + MORE_PADDING;
            DraggedGroupMore {
                rect: Rect::from_min_max(
                    pos2(bar.right() - width, bar.top()),
                    pos2(bar.right(), bar.bottom()),
                ),
                hidden,
                label,
            }
        });

        let ctx = ui.ctx();
        let layer = LayerId::new(Order::Tooltip, self.id.with("group ghost"));
        let painter = ctx.layer_painter(layer).with_clip_rect(ctx.content_rect());

        // The application's body, if it paints one: painted first, then taken out of the
        // layer and put back over the painter's ghost, so that its shadow stays under it.
        let start = ctx.graphics_mut(|g| g.entry(layer).next_idx().0);
        let body_painted = {
            let mut body_ui = Ui::new(
                ctx.clone(),
                self.id.with("ghost body"),
                UiBuilder::new().layer_id(layer).max_rect(body),
            );
            body_ui.set_clip_rect(body.intersect(ctx.content_rect()));
            body_ui.multiply_opacity(GHOST_OPACITY);
            self.dock_state
                .leaf(path)
                .ok()
                .and_then(|leaf| leaf.tabs.get(shown_tab))
                .is_some_and(|shown| tab_viewer.ghost_body(&mut body_ui, shown, body))
        };
        let body_shapes = ctx.graphics_mut(|g| {
            let list = g.entry(layer);
            let shapes: Vec<_> = list.all_entries().skip(start).cloned().collect();
            for i in start..start + shapes.len() {
                list.reset_shape(egui::layers::ShapeIdx(i));
            }
            shapes
        });

        let ghost = DraggedGroupPaint {
            rect,
            bar,
            body,
            tabs,
            more,
            count,
            body_painted,
            style,
            visuals: ui.visuals(),
        };
        if tab.is_some() {
            self.painter.dragged_tab(&painter, &ghost);
        } else {
            self.painter.dragged_group(&painter, &ghost);
        }

        if body_painted {
            ctx.graphics_mut(|g| {
                let list = g.entry(layer);
                for shape in body_shapes {
                    list.add(shape.clip_rect, shape.shape);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fitting_tabs;

    #[test]
    fn all_fit() {
        assert_eq!(fitting_tabs(&[10.0, 10.0, 10.0], 2.0, 100.0, 1), [0, 1, 2]);
    }

    #[test]
    fn the_first_that_fit() {
        assert_eq!(fitting_tabs(&[40.0, 40.0, 40.0], 0.0, 100.0, 0), [0, 1]);
    }

    #[test]
    fn the_active_one_is_always_shown() {
        assert_eq!(
            fitting_tabs(&[40.0, 40.0, 40.0, 40.0], 0.0, 100.0, 3),
            [0, 3]
        );
        assert_eq!(fitting_tabs(&[40.0, 40.0], 0.0, 30.0, 1), [1]);
    }
}
