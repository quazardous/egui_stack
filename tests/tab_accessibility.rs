//! Tabs and tab bars are exposed to AccessKit: role `Tab`, named by their title and `selected`
//! when active, inside a `TabList` per tab bar.

use egui::{Id, Ui, WidgetText, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
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

fn harness() -> Harness<'static, DockState<String>> {
    let mut dock_state = DockState::new(vec!["First".to_owned(), "Second".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    Harness::builder()
        .with_size([600.0, 300.0])
        // Only tabs are under test here.
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, dock_state: &mut DockState<String>| {
                DockArea::new(dock_state).show_inside(ui, &mut Viewer);
            },
            dock_state,
        )
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn tabs_have_a_role_a_name_and_a_selected_state() {
    let mut harness = harness();
    harness.run();

    assert_eq!(harness.query_all_by_role(Role::TabList).count(), 2);
    assert_eq!(harness.query_all_by_role(Role::Tab).count(), 3);

    for title in ["First", "Second", "Third"] {
        let tab = harness.get_by_role_and_label(Role::Tab, title);
        assert_eq!(
            tab.accesskit_node().parent().map(|parent| parent.role()),
            Some(Role::TabList),
            "{title} sits in a tab list"
        );
    }

    let selected = |harness: &Harness<'_, DockState<String>>, title: &str| {
        harness
            .get_by_role_and_label(Role::Tab, title)
            .accesskit_node()
            .is_selected()
    };
    assert_eq!(selected(&harness, "First"), Some(true));
    assert_eq!(selected(&harness, "Second"), Some(false));
    assert_eq!(selected(&harness, "Third"), Some(true));

    harness.get_by_role_and_label(Role::Tab, "Second").click();
    harness.run();

    assert_eq!(selected(&harness, "First"), Some(false));
    assert_eq!(selected(&harness, "Second"), Some(true));
}
