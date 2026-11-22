//! The layout changes under a drag: the app removes the dragged tab's leaf, or the leaf the
//! pointer is over, while the tab is still held. The dock forgets the stale paths: nothing panics,
//! and releasing moves nothing that is gone.

use egui::{Id, Pos2, Ui, WidgetText, accesskit::Role, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, NodeIndex, TabViewer};

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

/// `First` and `Second` in the left leaf, `Third` and `Fourth` in the right one.
fn harness() -> Harness<'static, DockState<String>> {
    let mut dock_state = DockState::new(vec!["First".to_owned(), "Second".to_owned()]);
    dock_state.main_surface_mut().split_right(
        NodeIndex::root(),
        0.5,
        vec!["Third".to_owned(), "Fourth".to_owned()],
    );
    Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, dock_state: &mut DockState<String>| {
            DockArea::new(dock_state).show_inside(ui, &mut Viewer);
        },
        dock_state,
    )
}

fn tabs(dock_state: &DockState<String>) -> Vec<String> {
    dock_state
        .iter_all_tabs()
        .map(|(_, tab)| tab.clone())
        .collect()
}

/// Presses on `First` and drags it over the middle of the right leaf, without releasing.
fn drag_first_over_right_leaf(harness: &mut Harness<'_, DockState<String>>) -> Pos2 {
    harness.run();
    let start = harness
        .get_by_role_and_label(Role::Tab, "First")
        .rect()
        .center();
    let leaf = harness.state().main_surface()[NodeIndex(2)].rect().unwrap();
    let target = leaf.center();

    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=5 {
        harness.hover_at(start + vec2(0.0, 10.0 * step as f32));
        harness.step();
    }
    for _ in 0..5 {
        harness.hover_at(target);
        harness.step();
    }
    target
}

/// Removes the leaf at `node` of the main surface, the way an app closes a group.
fn remove_leaf(harness: &mut Harness<'_, DockState<String>>, node: usize) {
    harness
        .state_mut()
        .main_surface_mut()
        .remove_leaf(NodeIndex(node));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_dragged_tabs_leaf_removed_mid_drag() {
    let mut harness = harness();
    let target = drag_first_over_right_leaf(&mut harness);
    remove_leaf(&mut harness, 1);
    for _ in 0..3 {
        harness.hover_at(target + vec2(5.0, 5.0));
        harness.step();
    }
    harness.drop_at(target);
    harness.run_steps(5);
    assert_eq!(tabs(harness.state()), ["Third", "Fourth"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_hovered_leaf_removed_mid_drag() {
    let mut harness = harness();
    let target = drag_first_over_right_leaf(&mut harness);
    remove_leaf(&mut harness, 2);
    for _ in 0..3 {
        harness.hover_at(target + vec2(5.0, 5.0));
        harness.step();
    }
    harness.drop_at(target);
    harness.run_steps(5);
    let mut left = tabs(harness.state());
    left.sort();
    assert_eq!(left, ["First", "Second"]);
}
