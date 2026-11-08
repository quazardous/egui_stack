//! [`LayoutArea`]: a [`Layout`] drawn in a [`Ui`], with sashes between its
//! zones.

use std::collections::HashMap;

use egui::{
    Color32, CornerRadius, CursorIcon, EventFilter, Id, Key, Painter, Rect, Response, Sense,
    Shadow, Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo,
    accesskit::{Action, ActionData, Orientation, Role},
};

use super::fold::{self, Flyout, FlyoutPaint, StripPaint};
use super::solve::{along, span};
use super::{
    Dir, Fold, Laid, LaidZone, Layout, LayoutEvent, Shown, Solve, Zone, ZoneKind, ZoneName,
};
use crate::{PartState, SeparatorAxis};

/// How a [`LayoutArea`] looks and moves: sizes and colours its
/// [`LayoutPainter`] reads, and the keyboard steps of its sashes.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct LayoutStyle {
    /// A folded zone's strip breadth. By default 24 points.
    pub strip: f32,
    /// A sash's drawn thickness. By default 1 point.
    pub sash_width: f32,
    /// A sash's thickness for the pointer, centred on the boundary. By
    /// default 6 points.
    pub sash_grab: f32,
    /// A sash at rest. By default transparent: the boundary shows on hover.
    pub sash_idle: Color32,
    /// A sash hovered or focused.
    pub sash_hovered: Color32,
    /// A sash dragged.
    pub sash_dragged: Color32,
    /// What an arrow key moves a focused sash by, in points. By default 16.
    pub key_step: f32,
    /// What an arrow key with Shift moves a focused sash by. By default 48.
    pub large_key_step: f32,
    /// A strip's background.
    pub strip_fill: Color32,
    /// The line between a strip and its neighbours.
    pub strip_stroke: Stroke,
    /// A strip's grip, the button that unfolds or folds its zone.
    pub grip: Color32,
    /// A strip's grip hovered.
    pub grip_hovered: Color32,
    /// The background of a zone lying over its neighbour, and of a flyout.
    pub flyout_fill: Color32,
    /// Their border.
    pub flyout_stroke: Stroke,
    /// Their shadow.
    pub flyout_shadow: Shadow,
    /// A flyout's breadth before the user resizes it. By default 240.
    pub flyout_size: f32,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self::from_egui(&egui::Style::default())
    }
}

impl LayoutStyle {
    /// A style drawn from egui's: the sash takes the widgets' colours.
    pub fn from_egui(style: &egui::Style) -> Self {
        let widgets = &style.visuals.widgets;
        Self {
            strip: 24.0,
            sash_width: 1.0,
            sash_grab: 6.0,
            sash_idle: Color32::TRANSPARENT,
            sash_hovered: widgets.hovered.bg_stroke.color,
            sash_dragged: widgets.active.bg_stroke.color,
            key_step: 16.0,
            large_key_step: 48.0,
            strip_fill: style.visuals.faint_bg_color,
            strip_stroke: widgets.noninteractive.bg_stroke,
            grip: widgets.noninteractive.fg_stroke.color,
            grip_hovered: widgets.hovered.fg_stroke.color,
            flyout_fill: style.visuals.panel_fill,
            flyout_stroke: style.visuals.window_stroke,
            flyout_shadow: style.visuals.popup_shadow,
            flyout_size: 240.0,
        }
    }
}

/// A sash to paint: the boundary between two zones.
#[derive(Debug)]
#[non_exhaustive]
pub struct SashPaint<'a> {
    /// The sash, [`LayoutStyle::sash_width`] thick.
    pub rect: Rect,
    /// Which way it runs.
    pub axis: SeparatorAxis,
    /// Its state: hovered, focused, dragged.
    pub state: PartState,
    /// The zone it resizes.
    pub zone: &'a ZoneName,
    /// The layout's style.
    pub style: &'a LayoutStyle,
}

/// Paints a layout's parts. Every method has a default drawn from
/// [`LayoutStyle`]; a host overrides the ones it styles itself.
pub trait LayoutPainter {
    /// A sash between two zones.
    fn sash(&self, painter: &Painter, sash: &SashPaint<'_>) {
        let style = sash.style;
        let color = if sash.state.contains(PartState::DRAGGED) {
            style.sash_dragged
        } else if sash.state.contains(PartState::HOVERED) || sash.state.contains(PartState::FOCUSED)
        {
            style.sash_hovered
        } else {
            style.sash_idle
        };
        painter.rect_filled(sash.rect, CornerRadius::ZERO, color);
    }

    /// A folded zone's strip, with its grip (painted before the
    /// [`ZoneViewer::strip`] content).
    fn strip(&self, painter: &Painter, strip: &StripPaint<'_>) {
        let style = strip.style;
        painter.rect_filled(strip.rect, CornerRadius::ZERO, style.strip_fill);
        painter.rect_stroke(
            strip.rect,
            CornerRadius::ZERO,
            style.strip_stroke,
            StrokeKind::Inside,
        );
        let color = if strip.grip_state.contains(PartState::HOVERED)
            || strip.grip_state.contains(PartState::FOCUSED)
        {
            style.grip_hovered
        } else {
            style.grip
        };
        // Three dots across the strip.
        let centre = strip.grip.center();
        let gap = egui::vec2(4.0, 0.0);
        let gap = if strip.axis == SeparatorAxis::Vertical {
            gap.yx()
        } else {
            gap
        };
        for offset in [-1.0, 0.0, 1.0] {
            painter.circle_filled(centre + gap * offset, 1.5, color);
        }
    }

    /// The frame of a zone lying over its neighbour, or of a flyout: shadow,
    /// background and border, painted before its content.
    fn flyout_frame(&self, painter: &Painter, frame: &FlyoutPaint<'_>) {
        let style = frame.style;
        painter.add(style.flyout_shadow.as_shape(frame.rect, CornerRadius::ZERO));
        painter.rect(
            frame.rect,
            CornerRadius::ZERO,
            style.flyout_fill,
            style.flyout_stroke,
            StrokeKind::Inside,
        );
    }
}

/// The painter a [`LayoutArea`] uses unless given one: every default of
/// [`LayoutPainter`].
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultLayoutPainter;

impl LayoutPainter for DefaultLayoutPainter {}

/// What the application draws in a [`LayoutArea`].
pub trait ZoneViewer {
    /// A leaf zone's content, in its rect; also while it lies over its
    /// neighbour.
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName);

    /// A folded zone's strip, after the painter's background and grip: the
    /// application's icons, say. The `Ui` is the strip less its grip. By
    /// default nothing.
    fn strip(&mut self, ui: &mut Ui, zone: &ZoneName) {
        let _ = (ui, zone);
    }

    /// The content of the [`Flyout`] given to [`LayoutArea::flyout`]. By
    /// default nothing.
    fn flyout(&mut self, ui: &mut Ui, flyout: &Flyout) {
        let _ = (ui, flyout);
    }
}

/// Draws a [`Layout`] in a [`Ui`]: solves it in the `Ui`'s rect, calls the
/// [`ZoneViewer`] for each leaf zone shown, and puts a sash between
/// neighbouring zones.
///
/// A sash resizes the zone on its side away from the zone that takes the
/// rest (the one that grows, else the last one): dragged, or focused and
/// moved with the arrow keys (by [`LayoutStyle::key_step`], further with
/// Shift). It honours the zone's least and largest sizes and the least size
/// of the zone that takes the rest. Enter or a double click gives the zone
/// back its default size. Each sash is a splitter for assistive technology
/// ("Resize left"), whose value is the zone's size, and each leaf zone a
/// pane named after it.
///
/// The user's changes are written to the layout's [`ZoneState`](super::ZoneState)
/// at once and listed in the returned [`Laid::events`].
///
/// ```
/// use egui_stack::layout::{Content, Dir, Layout, LayoutArea, Size, Zone, ZoneName, ZoneViewer};
///
/// struct Viewer;
/// impl ZoneViewer for Viewer {
///     fn ui(&mut self, ui: &mut egui::Ui, zone: &ZoneName) {
///         ui.label(zone.to_string());
///     }
/// }
///
/// let mut layout = Layout::new(Zone::split(
///     "root",
///     Dir::Horizontal,
///     vec![
///         Zone::leaf("side", Content::App).with(|c| c.default = Some(Size::Points(200.0))),
///         Zone::leaf("main", Content::App).with(|c| c.grow = true),
///     ],
/// ));
/// # egui::__run_test_ui(|ui| {
/// let laid = LayoutArea::new(&mut layout).show_inside(ui, &mut Viewer);
/// # let _ = laid;
/// # });
/// ```
pub struct LayoutArea<'a> {
    layout: &'a mut Layout,
    id: Id,
    style: Option<LayoutStyle>,
    painter: &'a dyn LayoutPainter,
    flyout: Option<Flyout>,
}

impl<'a> LayoutArea<'a> {
    /// An area drawing `layout`.
    pub fn new(layout: &'a mut Layout) -> Self {
        Self {
            layout,
            id: Id::unique("egui_stack::LayoutArea"),
            style: None,
            painter: &DefaultLayoutPainter,
            flyout: None,
        }
    }

    /// Sets the area's id, which keeps its sashes' state. Two layouts shown
    /// at once need different ids.
    #[inline]
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    /// Sets the style; by default [`LayoutStyle::from_egui`] of the `Ui`'s.
    #[inline]
    pub fn style(mut self, style: LayoutStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the painter; by default [`DefaultLayoutPainter`].
    #[inline]
    pub fn painter(mut self, painter: &'a dyn LayoutPainter) -> Self {
        self.painter = painter;
        self
    }

    /// Shows a flyout: an item of a folded zone ([`Flyout::zone`]) over its
    /// neighbour, next to the zone's strip, drawn by
    /// [`ZoneViewer::flyout`]. The application keeps it open by giving it
    /// every frame, and drops it on [`LayoutEvent::FlyoutClosed`] (Escape, a
    /// click outside it, the strip's grip, or the zone unfolding).
    #[inline]
    pub fn flyout(mut self, flyout: Option<Flyout>) -> Self {
        self.flyout = flyout;
        self
    }

    /// Draws the layout in the `Ui`'s whole rect, and returns where each
    /// zone went with what the user changed.
    pub fn show_inside(self, ui: &mut Ui, viewer: &mut dyn ZoneViewer) -> Laid {
        let style = self
            .style
            .unwrap_or_else(|| LayoutStyle::from_egui(ui.style()));
        let rect = ui.max_rect();
        let content_id = self.id.with("content");
        let content: HashMap<ZoneName, f32> = ui
            .ctx()
            .data(|d| d.get_temp(content_id))
            .unwrap_or_default();
        let solve = Solve {
            strip: style.strip,
            content,
        };
        let laid = self.layout.solve(rect, &solve);

        // The zones' contents, then the sashes over them.
        let mut asked = HashMap::new();
        draw_zones(
            ui,
            self.id,
            &self.layout.root,
            None,
            &laid,
            viewer,
            &mut asked,
        );
        ui.ctx().data_mut(|d| d.insert_temp(content_id, asked));

        let mut events = Vec::new();
        let painter = self.painter;
        fold::strips(
            ui,
            self.id,
            self.layout,
            &laid,
            &style,
            painter,
            viewer,
            self.flyout.as_ref(),
            &mut events,
        );
        fold::overs(
            ui,
            self.id,
            self.layout,
            &laid,
            &style,
            painter,
            viewer,
            &mut events,
        );
        let flyout_rect = self.flyout.as_ref().and_then(|flyout| {
            fold::flyout(
                ui,
                self.id,
                self.layout,
                &laid,
                flyout,
                &style,
                painter,
                viewer,
                &mut events,
            )
        });
        let sashes = sashes(&self.layout.root, &laid);
        for sash in sashes {
            sash_ui(
                ui,
                self.id,
                self.layout,
                &sash,
                &style,
                self.painter,
                &mut events,
            );
        }

        let mut laid = if events.is_empty() {
            laid
        } else {
            ui.ctx().request_repaint();
            self.layout.solve(rect, &solve)
        };
        laid.events = events;
        laid.flyout = flyout_rect;
        laid
    }
}

/// Draws the leaf zones under `zone` that are laid and open.
fn draw_zones(
    ui: &mut Ui,
    id: Id,
    zone: &Zone,
    parent: Option<Dir>,
    laid: &Laid,
    viewer: &mut dyn ZoneViewer,
    asked: &mut HashMap<ZoneName, f32>,
) {
    let Some(at) = laid.get(&zone.name.0) else {
        return;
    };
    match &zone.kind {
        ZoneKind::Split { dir, children } => {
            if at.shown == Shown::Open {
                for child in children {
                    draw_zones(ui, id, child, Some(*dir), laid, viewer, asked);
                }
            }
        }
        ZoneKind::Leaf { .. } => {
            if at.shown == Shown::Open {
                let size = zone_ui(ui, id, at, viewer);
                if zone.constraints.fit_content
                    && let Some(dir) = parent
                {
                    asked.insert(zone.name.clone(), along(dir, size));
                }
            }
        }
    }
}

/// Draws one zone's content in its rect, as a pane named after it; returns
/// the size its content took.
pub(super) fn zone_ui_at(
    ui: &mut Ui,
    id: Id,
    zone: &Zone,
    laid: &Laid,
    viewer: &mut dyn ZoneViewer,
) {
    let mut asked = HashMap::new();
    match &zone.kind {
        ZoneKind::Leaf { .. } => {
            if let Some(at) = laid.get(&zone.name.0) {
                zone_ui(ui, id, at, viewer);
            }
        }
        ZoneKind::Split { dir, children } => {
            for child in children {
                draw_zones(ui, id, child, Some(*dir), laid, viewer, &mut asked);
            }
        }
    }
}

pub(super) fn zone_ui(
    ui: &mut Ui,
    id: Id,
    at: &LaidZone,
    viewer: &mut dyn ZoneViewer,
) -> egui::Vec2 {
    let zone_id = id.with(("zone", &at.name.0));
    let response = ui.interact(at.rect, zone_id, Sense::hover());
    let label = at.name.0.clone();
    response.widget_info(|| WidgetInfo::labeled(Role::Pane, true, &label));
    let mut child = ui.new_child(
        UiBuilder::new()
            .id_salt(zone_id)
            .max_rect(at.rect)
            .layout(*ui.layout()),
    );
    child.set_clip_rect(at.rect.intersect(ui.clip_rect()));
    viewer.ui(&mut child, &at.name);
    child.min_rect().size()
}

/// A sash between two neighbouring open zones of a split.
struct Sash {
    dir: Dir,
    /// The boundary, across the split's breadth.
    rect: Rect,
    /// The zone it resizes.
    zone: ZoneName,
    /// Whether the zone is before the boundary (it grows as the sash moves
    /// right or down).
    before: bool,
    /// The zone's size now, and its bounds.
    size: f32,
    least: f32,
    most: f32,
}

/// The sashes of every split laid open under `zone`.
fn sashes(zone: &Zone, laid: &Laid) -> Vec<Sash> {
    let mut out = Vec::new();
    zone.visit(&mut |zone| {
        let ZoneKind::Split { dir, children } = &zone.kind else {
            return;
        };
        if laid
            .get(&zone.name.0)
            .is_none_or(|at| at.shown != Shown::Open)
        {
            return;
        }
        let shown: Vec<&LaidZone> = children
            .iter()
            .filter_map(|child| laid.get(&child.name.0))
            .filter(|at| at.shown != Shown::Hidden)
            .collect();
        let grows = |at: &LaidZone| {
            children
                .iter()
                .find(|c| c.name == at.name)
                .is_some_and(|c| c.constraints.grow)
        };
        let taker = shown
            .iter()
            .position(|at| grows(at))
            .or_else(|| shown.iter().rposition(|at| at.shown == Shown::Open));
        let Some(taker) = taker else {
            return;
        };
        let taker_slack = {
            let at = shown[taker];
            (along(*dir, at.rect.size()) - at.least).max(0.0)
        };
        for i in 0..shown.len().saturating_sub(1) {
            let (a, b) = (shown[i], shown[i + 1]);
            if a.shown != Shown::Open || b.shown != Shown::Open {
                continue;
            }
            let (resized, before) = if taker > i { (a, true) } else { (b, false) };
            let size = along(*dir, resized.rect.size());
            let boundary = along(*dir, a.rect.max.to_vec2());
            let rect = span(*dir, a.rect.union(b.rect), boundary, boundary);
            let most = resized.most.min(size + taker_slack).max(resized.least);
            out.push(Sash {
                dir: *dir,
                rect,
                zone: resized.name.clone(),
                before,
                size,
                least: resized.least,
                most,
            });
        }
    });
    out
}

fn sash_ui(
    ui: &mut Ui,
    id: Id,
    layout: &mut Layout,
    sash: &Sash,
    style: &LayoutStyle,
    painter: &dyn LayoutPainter,
    events: &mut Vec<LayoutEvent>,
) {
    let horizontal = sash.dir == Dir::Horizontal;
    let thick = |grow: f32| {
        let mid = along(sash.dir, sash.rect.min.to_vec2());
        span(sash.dir, sash.rect, mid - grow * 0.5, mid + grow * 0.5)
    };
    let sash_id = id.with(("sash", &sash.zone.0));
    let cursor = if horizontal {
        CursorIcon::ResizeHorizontal
    } else {
        CursorIcon::ResizeVertical
    };
    let response = ui
        .interact(thick(style.sash_grab), sash_id, Sense::click_and_drag())
        .on_hover_and_drag_cursor(cursor);

    // Moving the sash by `delta` (right or down) changes the zone by
    // `delta` before it, by `-delta` after it.
    let sign = if sash.before { 1.0 } else { -1.0 };
    let mut target = sash.size;
    let mut moved = false;
    let delta = along(sash.dir, response.drag_delta());
    if delta != 0.0 {
        let start_id = sash_id.with("start");
        // The drag's total, so a pointer past a bound comes back in step.
        let total: f32 = ui.ctx().data_mut(|d| {
            let start = d.get_temp_mut_or_insert_with(start_id, || (sash.size, 0.0_f32));
            start.1 += delta;
            start.0 + sign * start.1
        });
        target = total;
        moved = true;
    }
    // Where the drag would have put the zone, unbounded.
    let mut dragged_to = moved.then_some(target);
    let mut size_before = sash.size;
    if !response.dragged() {
        let start = ui
            .ctx()
            .data_mut(|d| d.remove_temp::<(f32, f32)>(sash_id.with("start")));
        if let Some((size, total)) = start {
            dragged_to = Some(size + sign * total);
            size_before = size;
        }
    }
    // Dragged under half its least size, a zone that folds folds on
    // release.
    let folds = layout
        .zone(&sash.zone.0)
        .is_some_and(|zone| zone.constraints.fold == Fold::ToStrip);
    let fold_below = sash.least * 0.5;
    if folds && response.drag_stopped() && dragged_to.is_some_and(|to| to < fold_below) {
        // It unfolds at the size it had before the drag.
        if let Some(zone) = layout.zone_mut(&sash.zone.0) {
            zone.state.size = Some(size_before);
        }
        layout.set_folded(&sash.zone.0, true);
        events.push(LayoutEvent::Folded {
            zone: sash.zone.clone(),
        });
        return;
    }
    let mut reset = response.double_clicked();
    if response.has_focus() {
        ui.memory_mut(|m| {
            m.set_focus_lock_filter(
                response.id,
                EventFilter {
                    horizontal_arrows: horizontal,
                    vertical_arrows: !horizontal,
                    tab: false,
                    escape: false,
                },
            );
        });
        let (back, forward) = if horizontal {
            (Key::ArrowLeft, Key::ArrowRight)
        } else {
            (Key::ArrowUp, Key::ArrowDown)
        };
        ui.input(|i| {
            let step = if i.modifiers.shift {
                style.large_key_step
            } else {
                style.key_step
            };
            if i.key_pressed(back) {
                target -= sign * step;
                moved = true;
            }
            if i.key_pressed(forward) {
                target += sign * step;
                moved = true;
            }
            reset |= i.key_pressed(Key::Enter);
        });
    }
    ui.input(|i| {
        let steps = i.num_accesskit_action_requests(response.id, Action::Increment) as f32
            - i.num_accesskit_action_requests(response.id, Action::Decrement) as f32;
        if steps != 0.0 {
            target += steps * style.key_step;
            moved = true;
        }
        if let Some(value) = i
            .accesskit_action_requests(response.id, Action::SetValue)
            .filter_map(|request| match request.data {
                Some(ActionData::NumericValue(value)) => Some(value as f32),
                _ => None,
            })
            .last()
        {
            target = value;
            moved = true;
        }
    });

    let mut size = sash.size;
    if reset {
        if let Some(zone) = layout.zone_mut(&sash.zone.0) {
            zone.state.size = None;
        }
        events.push(LayoutEvent::Reset {
            zone: sash.zone.clone(),
        });
    } else if moved {
        let clamped = target.clamp(sash.least, sash.most);
        if clamped != sash.size
            && let Some(zone) = layout.zone_mut(&sash.zone.0)
        {
            zone.state.size = Some(clamped);
            size = clamped;
            events.push(LayoutEvent::Resized {
                zone: sash.zone.clone(),
                size: clamped,
            });
        }
    }

    // Painted where it is after this frame's move.
    let mut drawn = sash.rect;
    let shift = sign * (size - sash.size);
    match sash.dir {
        Dir::Horizontal => drawn = drawn.translate(egui::vec2(shift, 0.0)),
        Dir::Vertical => drawn = drawn.translate(egui::vec2(0.0, shift)),
    }
    let mid = along(sash.dir, drawn.min.to_vec2());
    let drawn = span(
        sash.dir,
        drawn,
        mid - style.sash_width * 0.5,
        mid + style.sash_width * 0.5,
    );
    painter.sash(
        ui.painter(),
        &SashPaint {
            rect: drawn,
            axis: if horizontal {
                SeparatorAxis::Vertical
            } else {
                SeparatorAxis::Horizontal
            },
            state: PartState::NONE
                .with(PartState::HOVERED, response.hovered())
                .with(PartState::PRESSED, response.is_pointer_button_down_on())
                .with(PartState::FOCUSED, response.has_focus())
                .with(PartState::DRAGGED, response.dragged()),
            zone: &sash.zone,
            style,
        },
    );
    describe_sash(ui, &response, sash, size);
}

/// The sash as a splitter whose value is its zone's size.
fn describe_sash(ui: &Ui, response: &Response, sash: &Sash, size: f32) {
    let label = format!("Resize {}", sash.zone);
    let enabled = ui.is_enabled();
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(Role::Splitter, enabled, &label);
        info.value = Some(f64::from(size));
        info
    });
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_orientation(if sash.dir == Dir::Horizontal {
            Orientation::Vertical
        } else {
            Orientation::Horizontal
        });
        node.set_min_numeric_value(f64::from(sash.least));
        node.set_max_numeric_value(f64::from(sash.most));
        if sash.least < sash.most {
            node.add_action(Action::SetValue);
            if size < sash.most {
                node.add_action(Action::Increment);
            }
            if sash.least < size {
                node.add_action(Action::Decrement);
            }
        }
    });
}
