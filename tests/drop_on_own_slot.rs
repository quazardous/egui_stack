//! A tab dragged and released on its own slot, a move that changes nothing, goes back to its
//! look at rest: joined to its bar, its bottom corners square, not the rounded and outlined
//! shape of a dragged tab, and no dragged state is left on the frames after the release.

use egui::{Id, LayerId, Rect, Shape, Ui, WidgetText, accesskit::Role, epaint::ClippedShape, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use std::sync::Mutex;

use egui_stack::{
    DockArea, DockPainter, DockState, NodeIndex, PartState, StylePainter, TabPaint, TabViewer,
};

struct Viewer;

impl TabViewer for Viewer {
    type Tab = String;

    fn id(&mut self, tab: &mut String) -> Id {
        Id::unique(tab.as_str())
    }

    fn title(&mut self, tab: &mut String) -> WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut String) {
        ui.label(format!("Body of {tab}"));
    }
}

/// A tab painted: its title, rect, state, layer and the shapes the default painting added.
#[derive(Clone, Debug)]
struct Painted {
    title: String,
    rect: Rect,
    state: PartState,
    layer: LayerId,
    shapes: Vec<ClippedShape>,
}

/// A painter that records the tabs it paints, painted as by default.
#[derive(Default)]
struct Recorder(Mutex<Vec<Painted>>);

impl Recorder {
    /// Forgets what was recorded, before a frame.
    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }

    /// The paints of the tab titled `title` since the last [`clear`](Self::clear).
    fn of(&self, title: &str) -> Vec<Painted> {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter(|p| p.title == title)
            .cloned()
            .collect()
    }
}

impl DockPainter for Recorder {
    fn tab(&self, painter: &egui::Painter, tab: &TabPaint<'_>) {
        let layer = painter.layer_id();
        let entries = |ctx: &egui::Context| -> Vec<ClippedShape> {
            ctx.graphics(|g| {
                g.get(layer)
                    .map(|list| list.all_entries().cloned().collect())
                    .unwrap_or_default()
            })
        };
        let before = entries(painter.ctx()).len();
        StylePainter.tab(painter, tab);
        let shapes = entries(painter.ctx()).split_off(before);
        self.0.lock().unwrap().push(Painted {
            title: tab.title.text().to_owned(),
            rect: tab.rect,
            state: tab.state,
            layer,
            shapes,
        });
    }
}

/// The rect shapes of a paint: their rect, corner radius and stroke width.
fn rects(painted: &Painted) -> Vec<(Rect, egui::CornerRadius, f32)> {
    painted
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            Shape::Rect(rect) => Some((rect.rect, rect.corner_radius, rect.stroke.width)),
            _ => None,
        })
        .collect()
}

/// `First` and `Second` on the left, `Third` on the right; tabs rounded so a pill shows.
fn harness(recorder: &'static Recorder) -> Harness<'static, DockState<String>> {
    let mut dock_state = DockState::new(vec!["First".to_owned(), "Second".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    let mut harness = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        move |ui, dock_state: &mut DockState<String>| {
            let mut style = egui_stack::Style::from_egui(ui.style());
            for look in [
                &mut style.tab.active,
                &mut style.tab.inactive,
                &mut style.tab.focused,
                &mut style.tab.hovered,
                &mut style.tab.active_with_kb_focus,
                &mut style.tab.inactive_with_kb_focus,
                &mut style.tab.focused_with_kb_focus,
            ] {
                look.corner_radius = 6.into();
            }
            DockArea::new(dock_state)
                .style(style)
                .painter(recorder)
                .show_inside(ui, &mut Viewer);
        },
        dock_state,
    );
    harness.run();
    harness
}

/// How `Second` was painted: at rest, while held back on its own slot, and after the release.
struct Paints {
    rest: Painted,
    held: Vec<Vec<Painted>>,
    after: Vec<Vec<Painted>>,
}

/// Drags `Second` along `path` (offsets from its centre), holds it back on its own slot for a
/// few frames and releases it there; then lets the dock settle.
fn drop_on_own_slot(path: &[egui::Vec2]) -> Paints {
    let recorder: &'static Recorder = Box::leak(Box::default());
    let mut harness = harness(recorder);

    // At rest, `Second` selected (clicked), the pointer on it.
    let start = harness
        .get_by_role_and_label(Role::Tab, "Second")
        .rect()
        .center();
    harness.get_by_role_and_label(Role::Tab, "Second").click();
    harness.run();
    harness.hover_at(start);
    recorder.clear();
    harness.run();
    let rest = recorder
        .of("Second")
        .pop()
        .expect("Second is painted at rest");
    assert!(
        !rest.state.contains(PartState::DRAGGED),
        "at rest, the tab is not dragged"
    );

    harness.drag_at(start);
    harness.step();
    for offset in path {
        harness.hover_at(start + *offset);
        harness.step();
    }
    let mut held = Vec::new();
    for _ in 0..3 {
        recorder.clear();
        harness.hover_at(start);
        harness.step();
        held.push(recorder.of("Second"));
    }
    harness.drop_at(start);

    // The release frame, then the frames egui asks for, then a few more.
    let mut after = Vec::new();
    recorder.clear();
    harness.step();
    after.push(recorder.of("Second"));
    for _ in 0..4 {
        recorder.clear();
        harness.hover_at(start);
        harness.step();
        after.push(recorder.of("Second"));
    }

    assert_eq!(
        harness.state().main_surface()[NodeIndex(1)]
            .tabs()
            .unwrap()
            .to_vec(),
        ["First".to_owned(), "Second".to_owned()],
        "the drop on its own slot moved nothing"
    );
    Paints { rest, held, after }
}

/// Checks that each frame of `frames` paints the tab as at rest; `when` names the frames.
fn assert_at_rest(rest: &Painted, frames: &[Vec<Painted>], when: &str) {
    for (frame, paints) in frames.iter().enumerate() {
        // A step may run more than one pass: the last one is what shows.
        let painted = paints.last().unwrap_or_else(|| {
            panic!("frame {frame} {when}: the tab is painted");
        });
        assert!(
            !painted.state.contains(PartState::DRAGGED),
            "frame {frame} {when}: the tab is not painted as dragged: {:?}",
            painted.state
        );
        assert_eq!(
            painted.layer, rest.layer,
            "frame {frame} {when}: the tab is painted in its bar's layer"
        );
        assert_eq!(
            painted.rect, rest.rect,
            "frame {frame} {when}: the tab is back on its slot"
        );
        assert_eq!(
            rects(painted),
            rects(rest),
            "frame {frame} {when}: the tab paints the shapes it paints at rest \
             (square bottom corners joined to its bar, no pill outline)"
        );
    }
}

/// Checks the tab looks as at rest while held on its own slot and after its release there.
fn assert_looks_at_rest(paints: &Paints) {
    assert_at_rest(&paints.rest, &paints.held, "held back on its own slot");
    assert_at_rest(&paints.rest, &paints.after, "after the release");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_wiggled_and_dropped_on_its_own_slot_looks_as_at_rest() {
    // Dragged just past the click distance, then back, never leaving its slot.
    let paints = drop_on_own_slot(&[vec2(5.0, 0.0), vec2(9.0, 1.0), vec2(3.0, 0.0)]);
    assert_looks_at_rest(&paints);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_dragged_away_and_back_to_its_own_slot_looks_as_at_rest() {
    // Dragged away into the body, its ghost under the pointer, then back onto its slot.
    let path: Vec<_> = (1..=6)
        .map(|i| vec2(0.0, 20.0 * i as f32))
        .chain((0..=6).rev().map(|i| vec2(0.0, 20.0 * i as f32)))
        .collect();
    let paints = drop_on_own_slot(&path);
    assert_looks_at_rest(&paints);
}
