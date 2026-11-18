//! The ghost of a whole tab group being dragged, under the pointer.

use std::sync::Arc;

use egui::{Galley, LayerId, Order, Pos2, Rect, TextStyle, Ui, Vec2, WidgetText, pos2, vec2};

use crate::{
    DockArea, DraggedGroupMore, DraggedGroupPaint, DraggedGroupTab, NodePath, PartState,
    TabMeasure, TabViewer,
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
    /// Paints the ghost of the group at `path` under `pointer`, through
    /// [`DockPainter::dragged_group`](crate::DockPainter::dragged_group), on a layer above the
    /// drop overlay.
    pub(super) fn paint_group_ghost(
        &mut self,
        ui: &Ui,
        path: NodePath,
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
        let count = leaf.tabs.len();
        if count == 0 {
            return;
        }
        let active = leaf.active.0;
        let bar_height = style.tab_bar.height;
        let size = vec2(
            leaf_size.x.clamp(GHOST_MIN.x, GHOST_MAX.x),
            leaf_size.y.clamp(
                GHOST_MIN.y.max(bar_height + 24.0),
                GHOST_MAX.y.max(bar_height + 24.0),
            ),
        );
        let rect = Rect::from_min_size(pointer - vec2(GRAB_X, bar_height / 2.0), size);
        let bar = Rect::from_min_size(rect.min, vec2(size.x, bar_height));
        let body = Rect::from_min_max(pos2(rect.left(), bar.bottom()), rect.max);

        let layout = |text: WidgetText| -> Arc<Galley> {
            text.into_galley(ui, None, f32::INFINITY, TextStyle::Button)
        };
        let titles: Vec<Arc<Galley>> = leaf
            .tabs
            .iter_mut()
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

        let painter = ui
            .ctx()
            .layer_painter(LayerId::new(Order::Tooltip, self.id.with("group ghost")))
            .with_clip_rect(ui.ctx().content_rect());
        self.painter.dragged_group(
            &painter,
            &DraggedGroupPaint {
                rect,
                bar,
                body,
                tabs,
                more,
                count,
                style,
                visuals: ui.visuals(),
            },
        );
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
