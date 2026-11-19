//! Tabs and tab bars are exposed to AccessKit: role `Tab`, named by their title and `selected`
//! when active, inside a named `TabList` per tab bar; the active tab controls its body, a
//! `TabPanel` the tab names.

use egui::{Id, Ui, WidgetText, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::{DockArea, DockRules, DockSpace, DockState, NodeIndex, TabViewer};

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

    fn dock_title(&mut self, dock: &str) -> Option<WidgetText> {
        (dock == "side").then(|| "Side panel".into())
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

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_active_tab_controls_its_panel_which_it_names() {
    let mut harness = harness();
    harness.run();

    for title in ["First", "Third"] {
        let tab = harness.get_by_role_and_label(Role::Tab, title);
        let tab = tab.accesskit_node();
        let panels: Vec<_> = tab.controls().collect();
        assert_eq!(panels.len(), 1, "{title} controls one panel");
        let panel = &panels[0];
        assert_eq!(panel.role(), Role::TabPanel);
        let named_by: Vec<_> = panel.labelled_by().map(|node| node.id()).collect();
        assert_eq!(
            named_by,
            vec![tab.id()],
            "{title}'s panel is named by its tab"
        );
        assert_eq!(panel.label().as_deref(), Some(title));

        let label = format!("Body of {title}");
        let body = harness.get_by_label(&label);
        let body = body.accesskit_node();
        let mut ancestor = body.parent();
        let mut inside = false;
        while let Some(node) = ancestor {
            if node.id() == panel.id() {
                inside = true;
                break;
            }
            ancestor = node.parent();
        }
        assert!(inside, "the body of {title} lies inside its panel");
    }
    let inactive = harness.get_by_role_and_label(Role::Tab, "Second");
    assert_eq!(inactive.accesskit_node().controls().count(), 0);
    assert_eq!(harness.query_all_by_role(Role::TabPanel).count(), 2);

    for list in harness.query_all_by_role(Role::TabList) {
        assert_eq!(list.accesskit_node().label().as_deref(), Some("Tabs"));
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_docks_tab_list_takes_the_docks_title() {
    let mut space = DockSpace::new();
    space.add_dock("side", DockRules::default(), vec!["Files".to_owned()]);
    space.add_dock("other", DockRules::default(), vec!["Log".to_owned()]);
    let mut harness = Harness::builder()
        .with_size([600.0, 300.0])
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, space: &mut DockSpace<String>| {
                ui.columns(2, |columns| {
                    DockArea::dock(space, "side").show_inside(&mut columns[0], &mut Viewer);
                    DockArea::dock(space, "other").show_inside(&mut columns[1], &mut Viewer);
                });
            },
            space,
        );
    harness.run();

    let mut names: Vec<_> = harness
        .query_all_by_role(Role::TabList)
        .map(|list| list.accesskit_node().label())
        .collect();
    names.sort();
    assert_eq!(
        names,
        vec![Some("Side panel".to_owned()), Some("Tabs".to_owned())]
    );
}
