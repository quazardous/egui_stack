//! Escape during a tab drag cancels it: the layout stays as it was, even once the tab is released.
//!
//! egui aborts any drag on Escape; this pins that the dock follows, dropping nothing on release.

use egui::{Id, Key, Pos2, Ui, WidgetText, accesskit::Role, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, NodeIndex, TabPath, TabViewer};

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

fn harness() -> Harness<'static, DockState<String>> {
    let mut dock_state = DockState::new(vec!["First".to_owned(), "Second".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, dock_state: &mut DockState<String>| {
            DockArea::new(dock_state).show_inside(ui, &mut Viewer);
        },
        dock_state,
    )
}

/// Every tab and where it is, windows included.
fn layout(dock_state: &DockState<String>) -> Vec<(TabPath, String)> {
    dock_state
        .iter_all_tabs()
        .map(|(path, tab)| (path, tab.clone()))
        .collect()
}

/// Presses on `First` and drags it over the middle of the leaf holding `Third`, without releasing.
fn drag_first_over_third(harness: &mut Harness<'_, DockState<String>>) -> Pos2 {
    harness.run();
    let start = harness
        .get_by_role_and_label(Role::Tab, "First")
        .rect()
        .center();
    let third = harness.get_by_role_and_label(Role::Tab, "Third").rect();
    let leaf = harness.state().main_surface()[NodeIndex(2)].rect().unwrap();
    let target = Pos2::new(leaf.center().x, third.bottom() + 150.0);

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

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_released_drag_moves_the_tab() {
    let mut harness = harness();
    let target = drag_first_over_third(&mut harness);
    harness.drop_at(target);
    harness.run_steps(5);
    let tabs: Vec<_> = layout(harness.state())
        .into_iter()
        .map(|(_, tab)| tab)
        .collect();
    assert_eq!(tabs, ["Second", "Third", "First"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn escape_cancels_the_drag() {
    let mut harness = harness();
    harness.run();
    let before = layout(harness.state());

    let target = drag_first_over_third(&mut harness);
    harness.key_press(Key::Escape);
    harness.step();
    // Moving on and releasing after Escape changes nothing.
    harness.hover_at(target + vec2(5.0, 5.0));
    harness.step();
    harness.drop_at(target);
    harness.run_steps(5);

    assert_eq!(layout(harness.state()), before);
}
