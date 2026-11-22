//! `TabFocus::Roving`: one Tab stop per tab bar; arrows, Home and End move along it.

use egui::{Id, Key, Ui, WidgetText, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::{DockArea, DockState, TabFocus, TabViewer};

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

fn harness(tab_focus: TabFocus) -> Harness<'static, DockState<String>> {
    Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        move |ui, dock_state: &mut DockState<String>| {
            DockArea::new(dock_state)
                .tab_focus(tab_focus)
                .show_inside(ui, &mut Viewer);
        },
        DockState::new(vec!["One".to_owned(), "Two".to_owned(), "Three".to_owned()]),
    )
}

/// The name of every widget the Tab key stops on, once round.
fn tab_stops(harness: &mut Harness<'_, DockState<String>>) -> Vec<String> {
    harness.run();
    let mut stops = Vec::new();
    for _ in 0..12 {
        harness.key_press(Key::Tab);
        harness.run();
        let focused = harness
            .root()
            .children_recursive()
            .find(|node| node.is_focused())
            .and_then(|node| node.accesskit_node().label().map(|label| label.to_string()));
        if let Some(name) = focused {
            if stops.first() == Some(&name) {
                break;
            }
            stops.push(name);
        }
    }
    stops
}

fn selected(harness: &Harness<'_, DockState<String>>) -> Vec<String> {
    harness
        .query_all_by_role(Role::Tab)
        .filter(|tab| tab.accesskit_node().is_selected() == Some(true))
        .filter_map(|tab| tab.accesskit_node().label().map(|label| label.to_string()))
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn each_tab_is_a_tab_stop_by_default() {
    let stops = tab_stops(&mut harness(TabFocus::Each));
    for tab in ["One", "Two", "Three"] {
        assert!(stops.iter().any(|stop| stop == tab), "{tab} in {stops:?}");
    }
    assert!(stops.iter().any(|stop| stop == "Close tab"), "{stops:?}");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn roving_focus_has_one_tab_stop_per_bar() {
    let stops = tab_stops(&mut harness(TabFocus::Roving));
    assert!(stops.iter().any(|stop| stop == "One"), "{stops:?}");
    for not_a_stop in ["Two", "Three", "Close tab"] {
        assert!(
            stops.iter().all(|stop| stop != not_a_stop),
            "{not_a_stop} in {stops:?}"
        );
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn arrows_home_and_end_move_along_the_bar() {
    let mut harness = harness(TabFocus::Roving);
    harness.run();
    harness.get_by_role_and_label(Role::Tab, "One").focus();
    harness.run();

    let mut press = |key| {
        harness.key_press(key);
        harness.run();
        let focused = harness
            .query_all_by_role(Role::Tab)
            .find(|tab| tab.is_focused())
            .and_then(|tab| tab.accesskit_node().label().map(|label| label.to_string()));
        (focused, selected(&harness))
    };
    let at = |name: &str| (Some(name.to_owned()), vec![name.to_owned()]);

    assert_eq!(press(Key::ArrowRight), at("Two"));
    assert_eq!(press(Key::End), at("Three"));
    assert_eq!(press(Key::ArrowRight), at("One"));
    assert_eq!(press(Key::ArrowLeft), at("Three"));
    assert_eq!(press(Key::Home), at("One"));
}
