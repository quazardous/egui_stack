//! `WindowState::rect` and `WindowState::dragged` follow the window as it is shown and dragged.

use egui::{Id, Pos2, Rect, Ui, WidgetText, vec2};
use egui_kittest::Harness;
use egui_stack::{DockArea, DockState, SurfaceIndex, TabViewer};

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

    fn scroll_bars(&self, _tab: &String) -> [bool; 2] {
        // No scroll area, so a drag on the body moves the window.
        [false, false]
    }
}

fn harness() -> (Harness<'static, DockState<String>>, SurfaceIndex) {
    let mut dock_state = DockState::new(vec!["Main".to_owned()]);
    let window = dock_state.add_window(vec!["Floating".to_owned()]);
    dock_state
        .get_window_state_mut(window)
        .unwrap()
        .set_position(Pos2::new(300.0, 150.0))
        .set_size(vec2(200.0, 120.0));
    let harness = Harness::builder().with_size([800.0, 500.0]).build_ui_state(
        |ui, dock_state: &mut DockState<String>| {
            DockArea::new(dock_state).show_inside(ui, &mut Viewer);
        },
        dock_state,
    );
    (harness, window)
}

fn window_state(harness: &Harness<'_, DockState<String>>, window: SurfaceIndex) -> (Rect, bool) {
    let state = harness.state().get_window_state(window).unwrap();
    (state.rect(), state.dragged())
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn rect_and_dragged_follow_the_window() {
    let (mut harness, window) = harness();
    harness.run();
    let (rect, dragged) = window_state(&harness, window);
    assert_ne!(rect, Rect::NOTHING);
    assert_eq!(rect.size(), vec2(200.0, 120.0));
    assert!(!dragged);

    // Drag the window by an empty part of its body.
    let start = Pos2::new(rect.center().x, rect.bottom() - 20.0);
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=4 {
        harness.hover_at(start + vec2(10.0 * step as f32, 0.0));
        harness.step();
    }
    let (moving, dragged) = window_state(&harness, window);
    assert!(dragged, "the window is dragged");
    assert!(moving.min.x > rect.min.x, "{moving:?}");

    harness.drop_at(start + vec2(40.0, 0.0));
    harness.run_steps(3);
    let (moved, dragged) = window_state(&harness, window);
    assert!(!dragged);
    assert_eq!(moved.min, rect.min + vec2(40.0, 0.0));
}
