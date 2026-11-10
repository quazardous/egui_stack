//! Folded zones: their strips, the zones lying over their neighbour, and
//! the flyout.

use egui::{CursorIcon, Id, Key, Order, Rect, Sense, Ui, UiBuilder, WidgetInfo, accesskit::Role};

use super::area::{
    LayoutPainter, LayoutStyle, Names, SnapGuidePaint, SnapKind, ZoneViewer, paint_guide,
    zone_ui_at,
};
use super::solve::{along, span};
use super::{Dir, Laid, LaidZone, Layout, LayoutEvent, Shown, Zone, ZoneName};
use crate::{PartState, SeparatorAxis};

/// A flyout: one item of a folded zone, shown over the neighbouring zone
/// next to the zone's strip, without moving the layout. The application
/// says what it is ([`ZoneViewer::flyout`]) and when to open it
/// ([`LayoutArea::flyout`](super::LayoutArea::flyout)).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Flyout {
    /// The application's key (a panel), under which its breadth is kept
    /// ([`Layout::flyouts`]).
    pub key: String,
    /// The folded zone whose strip it opens from.
    pub zone: ZoneName,
}

impl Flyout {
    /// The flyout `key` of the folded zone `zone`.
    pub fn new(key: impl Into<String>, zone: impl Into<ZoneName>) -> Self {
        Self {
            key: key.into(),
            zone: zone.into(),
        }
    }
}

/// A folded zone's strip to paint.
#[derive(Debug)]
#[non_exhaustive]
pub struct StripPaint<'a> {
    /// The strip.
    pub rect: Rect,
    /// Its grip, the button at its start that unfolds (or folds back) its
    /// zone.
    pub grip: Rect,
    /// Which way the strip runs: [`SeparatorAxis::Vertical`] for a column
    /// in a row of zones.
    pub axis: SeparatorAxis,
    /// The grip's state.
    pub grip_state: PartState,
    /// The zone.
    pub zone: &'a ZoneName,
    /// How the zone is shown: folded, or lying over its neighbour.
    pub shown: Shown,
    /// The layout's style.
    pub style: &'a LayoutStyle,
}

/// The frame of a zone lying over its neighbour, or of a flyout.
#[derive(Debug)]
#[non_exhaustive]
pub struct FlyoutPaint<'a> {
    /// Its rect.
    pub rect: Rect,
    /// The zone it opens from.
    pub zone: &'a ZoneName,
    /// The flyout's key, for a flyout; `None` for a zone lying over.
    pub key: Option<&'a str>,
    /// The layout's style.
    pub style: &'a LayoutStyle,
}

/// The direction of the split holding the zone named `name`.
fn parent_dir(root: &Zone, name: &str) -> Option<Dir> {
    let mut found = None;
    root.visit(&mut |zone| {
        if let super::ZoneKind::Split { dir, children } = &zone.kind
            && children.iter().any(|c| c.name == *name)
        {
            found = Some(*dir);
        }
    });
    found
}

/// The name of the split holding the zone named `name`.
fn parent_name(root: &Zone, name: &str) -> Option<ZoneName> {
    let mut found = None;
    root.visit(&mut |zone| {
        if zone.children().iter().any(|c| c.name == *name) {
            found = Some(zone.name.clone());
        }
    });
    found
}

/// Whether a press this frame fell outside every rect of `inside`.
fn pressed_outside(ui: &Ui, inside: &[Rect]) -> bool {
    ui.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .press_origin()
                .is_some_and(|pos| !inside.iter().any(|rect| rect.contains(pos)))
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "the area's parts, borrowed apart"
)]
pub(super) fn strips(
    ui: &mut Ui,
    id: Id,
    layout: &mut Layout,
    laid: &Laid,
    style: &LayoutStyle,
    painter: &dyn LayoutPainter,
    viewer: &mut dyn ZoneViewer,
    names: &Names<'_>,
    flyout: Option<&Flyout>,
    events: &mut Vec<LayoutEvent>,
) {
    for at in &laid.zones {
        let Some(strip) = at.strip else {
            continue;
        };
        let dir = parent_dir(&layout.root, &at.name.0).unwrap_or_default();
        let breadth = along(dir, strip.size());
        let grip = Rect::from_min_size(strip.min, egui::vec2(breadth, breadth)).intersect(strip);
        let response = ui.interact(grip, id.with(("grip", &at.name.0)), Sense::click());
        let label = match at.shown {
            Shown::Over => names.fold(&at.name),
            _ => names.unfold(&at.name),
        };
        response.widget_info(|| {
            WidgetInfo::labeled(Role::Button, at.shown != Shown::AutoFolded, &label)
        });
        let axis = match dir {
            Dir::Horizontal => SeparatorAxis::Vertical,
            Dir::Vertical => SeparatorAxis::Horizontal,
        };
        painter.strip(
            ui.painter(),
            &StripPaint {
                rect: strip,
                grip,
                axis,
                grip_state: PartState::NONE
                    .with(PartState::HOVERED, response.hovered())
                    .with(PartState::PRESSED, response.is_pointer_button_down_on())
                    .with(PartState::FOCUSED, response.has_focus()),
                zone: &at.name,
                shown: at.shown,
                style,
            },
        );
        let rest = match dir {
            Dir::Horizontal => Rect::from_min_max(egui::pos2(strip.min.x, grip.max.y), strip.max),
            Dir::Vertical => Rect::from_min_max(egui::pos2(grip.max.x, strip.min.y), strip.max),
        };
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt(id.with(("strip", &at.name.0)))
                .max_rect(rest)
                .layout(match dir {
                    Dir::Horizontal => egui::Layout::top_down(egui::Align::Center),
                    Dir::Vertical => egui::Layout::left_to_right(egui::Align::Center),
                }),
        );
        child.set_clip_rect(rest.intersect(ui.clip_rect()));
        viewer.strip(&mut child, &at.name);

        if at.shown == Shown::Folded
            && style.strip_drag
            && drag_out(ui, id, layout, laid, at, strip, dir, style, painter)
        {
            events.push(LayoutEvent::Unfolded {
                zone: at.name.clone(),
            });
            continue;
        }

        if response.clicked() {
            match (flyout, at.shown) {
                (Some(flyout), _) if flyout.zone == at.name => {
                    events.push(LayoutEvent::FlyoutClosed {
                        key: flyout.key.clone(),
                    });
                }
                (_, Shown::Folded) => {
                    layout.set_folded(&at.name.0, false);
                    events.push(LayoutEvent::Unfolded {
                        zone: at.name.clone(),
                    });
                }
                (_, Shown::Over) => {
                    layout.set_folded(&at.name.0, true);
                    events.push(LayoutEvent::Folded {
                        zone: at.name.clone(),
                    });
                }
                _ => {}
            }
        }
    }
}

/// Draws the zones lying over their neighbour, in front of the others; a
/// press outside one, or Escape, folds it back.
#[expect(
    clippy::too_many_arguments,
    reason = "the area's parts, borrowed apart"
)]
pub(super) fn overs(
    ui: &mut Ui,
    id: Id,
    layout: &mut Layout,
    laid: &Laid,
    style: &LayoutStyle,
    painter: &dyn LayoutPainter,
    viewer: &mut dyn ZoneViewer,
    names: &Names<'_>,
    events: &mut Vec<LayoutEvent>,
) {
    for at in laid.zones.iter().filter(|at| at.shown == Shown::Over) {
        let Some(zone) = layout.zone(&at.name.0).cloned() else {
            continue;
        };
        let rect = at.rect;
        let area_id = id.with(("over", &at.name.0));
        egui::Area::new(area_id)
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            .constrain(false)
            .movable(false)
            .show(ui.ctx(), |ui| {
                ui.allocate_rect(rect, Sense::click());
                painter.flyout_frame(
                    ui.painter(),
                    &FlyoutPaint {
                        rect,
                        zone: &at.name,
                        key: None,
                        style,
                    },
                );
                zone_ui_at(ui, id, &zone, laid, names, viewer);
            });
        let inside = [rect, at.strip.unwrap_or(Rect::NOTHING)];
        let escape = ui.input(|i| i.key_pressed(Key::Escape));
        if escape || pressed_outside(ui, &inside) {
            layout.set_folded(&at.name.0, true);
            events.push(LayoutEvent::Folded {
                zone: at.name.clone(),
            });
        }
    }
}

/// Draws the flyout next to its zone's strip, in front of the others, with
/// its free edge that resizes it; returns its rect. A press outside it,
/// Escape, or its zone no longer folded asks to close it.
#[expect(
    clippy::too_many_arguments,
    reason = "the area's parts, borrowed apart"
)]
pub(super) fn flyout(
    ui: &mut Ui,
    id: Id,
    layout: &mut Layout,
    laid: &Laid,
    flyout: &Flyout,
    style: &LayoutStyle,
    painter: &dyn LayoutPainter,
    viewer: &mut dyn ZoneViewer,
    names: &Names<'_>,
    events: &mut Vec<LayoutEvent>,
) -> Option<Rect> {
    let closed = || LayoutEvent::FlyoutClosed {
        key: flyout.key.clone(),
    };
    let at = laid.get(&flyout.zone.0);
    let Some((at, strip)) = at.and_then(|at| {
        matches!(at.shown, Shown::Folded | Shown::AutoFolded)
            .then_some(at)
            .zip(at.strip)
    }) else {
        events.push(closed());
        return None;
    };
    let dir = parent_dir(&layout.root, &at.name.0).unwrap_or_default();
    let parent = parent_name(&layout.root, &at.name.0)
        .and_then(|name| laid.rect(&name.0))
        .unwrap_or_else(|| ui.max_rect());
    let (start, end) = (
        along(dir, parent.min.to_vec2()),
        along(dir, parent.max.to_vec2()),
    );
    let towards_end = along(dir, strip.center().to_vec2()) < (start + end) * 0.5;
    let size = layout
        .flyouts
        .get(&flyout.key)
        .copied()
        .or_else(|| layout.zone(&at.name.0).and_then(|zone| zone.state.size))
        .unwrap_or(style.flyout_size);
    let rect_of = |size: f32| {
        if towards_end {
            let from = along(dir, strip.max.to_vec2());
            span(dir, parent, from, (from + size).min(end))
        } else {
            let to = along(dir, strip.min.to_vec2());
            span(dir, parent, (to - size).max(start), to)
        }
    };
    let rect = rect_of(size);
    let free_edge = if towards_end {
        along(dir, rect.max.to_vec2())
    } else {
        along(dir, rect.min.to_vec2())
    };
    let edge = span(
        dir,
        rect,
        free_edge - style.sash_grab * 0.5,
        free_edge + style.sash_grab * 0.5,
    );
    let area_id = id.with(("flyout", &flyout.key));
    let label = names.resize_flyout(&viewer.flyout_title(flyout));
    let mut resized = None;
    egui::Area::new(area_id)
        .order(Order::Foreground)
        .fixed_pos(rect.min)
        .constrain(false)
        .movable(false)
        .show(ui.ctx(), |ui| {
            ui.allocate_rect(rect, Sense::click());
            painter.flyout_frame(
                ui.painter(),
                &FlyoutPaint {
                    rect,
                    zone: &at.name,
                    key: Some(&flyout.key),
                    style,
                },
            );
            let mut child = ui.new_child(
                UiBuilder::new()
                    .id_salt(area_id.with("content"))
                    .max_rect(rect),
            );
            child.set_clip_rect(rect);
            viewer.flyout(&mut child, flyout);

            let cursor = match dir {
                Dir::Horizontal => egui::CursorIcon::ResizeHorizontal,
                Dir::Vertical => egui::CursorIcon::ResizeVertical,
            };
            let response = ui
                .interact(edge, area_id.with("edge"), Sense::drag())
                .on_hover_and_drag_cursor(cursor);
            response.widget_info(|| WidgetInfo::labeled(Role::Splitter, true, &label));
            let delta = along(dir, response.drag_delta());
            if delta != 0.0 {
                let sign = if towards_end { 1.0 } else { -1.0 };
                let least = style.strip * 2.0;
                let new = (size + sign * delta).clamp(least, (end - start).max(least));
                if new != size {
                    resized = Some(new);
                }
            }
        });
    if let Some(size) = resized {
        layout.flyouts.insert(flyout.key.clone(), size);
        events.push(LayoutEvent::FlyoutResized {
            key: flyout.key.clone(),
            size,
        });
    }
    let escape = ui.input(|i| i.key_pressed(Key::Escape));
    if escape || pressed_outside(ui, &[rect, strip, edge]) {
        events.push(closed());
    }
    Some(rect)
}

/// Whether the folded zone `at`'s strip opens towards the end of its parent
/// split (a column on the left opens rightwards).
fn towards_end(
    layout: &Layout,
    laid: &Laid,
    ui: &Ui,
    at: &LaidZone,
    strip: Rect,
    dir: Dir,
) -> bool {
    let parent = parent_name(&layout.root, &at.name.0)
        .and_then(|name| laid.rect(&name.0))
        .unwrap_or_else(|| ui.max_rect());
    let (start, end) = (
        along(dir, parent.min.to_vec2()),
        along(dir, parent.max.to_vec2()),
    );
    along(dir, strip.center().to_vec2()) < (start + end) * 0.5
}

/// The edge of a folded zone's strip facing its neighbour, dragged: past
/// half the zone's least size, the zone unfolds on release, at the size
/// dragged to, with a guide line meanwhile. Returns whether it unfolded.
#[expect(
    clippy::too_many_arguments,
    reason = "the area's parts, borrowed apart"
)]
fn drag_out(
    ui: &mut Ui,
    id: Id,
    layout: &mut Layout,
    laid: &Laid,
    at: &LaidZone,
    strip: Rect,
    dir: Dir,
    style: &LayoutStyle,
    painter: &dyn LayoutPainter,
) -> bool {
    let towards_end = towards_end(layout, laid, ui, at, strip, dir);
    let edge = if towards_end {
        along(dir, strip.max.to_vec2())
    } else {
        along(dir, strip.min.to_vec2())
    };
    let band = span(
        dir,
        strip,
        edge - style.sash_grab * 0.5,
        edge + style.sash_grab * 0.5,
    );
    let edge_id = id.with(("strip_edge", &at.name.0));
    // A drag only: the grip is the strip's keyboard stop.
    let cursor = match dir {
        Dir::Horizontal => CursorIcon::ResizeHorizontal,
        Dir::Vertical => CursorIcon::ResizeVertical,
    };
    let response = ui
        .interact(band, edge_id, Sense::DRAG)
        .on_hover_and_drag_cursor(cursor);
    let sign = if towards_end { 1.0 } else { -1.0 };
    let delta = along(dir, response.drag_delta());
    let total_id = edge_id.with("total");
    let breadth = along(dir, strip.size());
    let mut total = None;
    if response.dragged() || delta != 0.0 {
        total = Some(ui.ctx().data_mut(|d| {
            let total = d.get_temp_mut_or_insert_with(total_id, || 0.0_f32);
            *total += sign * delta;
            *total
        }));
    }
    if !response.dragged()
        && let Some(done) = ui.ctx().data_mut(|d| d.remove_temp::<f32>(total_id))
    {
        total = Some(done);
    }
    let Some(total) = total else {
        return false;
    };
    // The zone's size were it unfolded at the pointer: from the strip's far
    // edge to it.
    let extent = breadth + total;
    let unfolds = total > 0.0 && extent >= at.least * 0.5;
    let size = extent.clamp(at.least, at.most.max(at.least));
    if response.dragged() && unfolds {
        // Across the strip's breadth, the split's.
        let at_size = edge - sign * breadth + sign * size;
        let line = span(dir, strip, at_size, at_size);
        paint_guide(
            ui,
            id,
            painter,
            &SnapGuidePaint {
                line: [line.min, line.max],
                axis: match dir {
                    Dir::Horizontal => SeparatorAxis::Vertical,
                    Dir::Vertical => SeparatorAxis::Horizontal,
                },
                kind: SnapKind::Unfold,
                zone: &at.name,
                style,
            },
        );
    }
    if response.drag_stopped() && unfolds {
        if let Some(zone) = layout.zone_mut(&at.name.0) {
            zone.state.size = Some(size);
        }
        layout.set_folded(&at.name.0, false);
        return true;
    }
    false
}
