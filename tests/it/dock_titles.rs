//! `TabViewer::dock_title`: the Move group menu names the docks of a space by the viewer's
//! titles, and by their names where the viewer gives none, except a float's dock the space
//! named itself (`float-N`), named by its window's title: its active tab's.

use egui::{
    Event, Id, Modifiers, PointerButton, Rect, Ui, UiBuilder, WidgetText, accesskit::Role, pos2,
};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{
    DockArea, DockRules, DockSpace, NodeIndex, NodePath, TabIndex, TabPath, TabViewer,
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

    fn dock_title(&mut self, dock: &str) -> Option<WidgetText> {
        match dock {
            "start_side" => Some("Left panel".into()),
            "float-2" => Some("Scratch".into()),
            _ => None,
        }
    }
}

/// Where each dock is drawn on the 900 × 400 screen.
fn dock_rect(name: &str) -> Rect {
    match name {
        "start_side" => Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 400.0)),
        "centre" => Rect::from_min_max(pos2(200.0, 0.0), pos2(700.0, 400.0)),
        "end_side" => Rect::from_min_max(pos2(700.0, 0.0), pos2(900.0, 400.0)),
        _ => unreachable!(),
    }
}

const DOCKS: [&str; 3] = ["start_side", "centre", "end_side"];

/// `Files` in `start_side`, `Editor` in `centre`, nothing in `end_side`.
fn harness() -> Harness<'static, DockSpace<String>> {
    harness_of(space())
}

fn space() -> DockSpace<String> {
    let mut space = DockSpace::new();
    space.add_dock("start_side", DockRules::default(), vec!["Files".to_owned()]);
    space.add_dock("centre", DockRules::default(), vec!["Editor".to_owned()]);
    space.add_dock("end_side", DockRules::default(), vec![]);
    space
}

/// Adds the dock `name` holding `tabs`, then moves its group out to a float of the space's
/// making; returns the float's dock.
fn float_out(space: &mut DockSpace<String>, name: &str, tabs: &[&str]) -> String {
    space.add_dock(
        name,
        DockRules::default(),
        tabs.iter().map(|&tab| tab.to_owned()).collect(),
    );
    let surface = space.dock_surface(name).unwrap();
    space
        .float_out_group(
            NodePath::new(surface, NodeIndex::root()),
            Rect::from_min_size(pos2(100.0, 100.0), egui::vec2(200.0, 150.0)),
            |tab| Id::unique(tab.as_str()),
        )
        .unwrap()
}

fn harness_of(space: DockSpace<String>) -> Harness<'static, DockSpace<String>> {
    let mut harness = Harness::builder().with_size([900.0, 400.0]).build_ui_state(
        |ui, space: &mut DockSpace<String>| {
            for name in DOCKS {
                ui.scope_builder(UiBuilder::new().max_rect(dock_rect(name)), |ui| {
                    DockArea::dock(space, name).show_inside(ui, &mut Viewer);
                });
            }
        },
        space,
    );
    harness.run();
    harness
}

/// Opens `Editor`'s context menu, then its Move group submenu.
fn open_move_group_menu(harness: &mut Harness<'_, DockSpace<String>>) {
    let rect = harness.get_by_role_and_label(Role::Tab, "Editor").rect();
    let pos = pos2(rect.left() + 6.0, rect.center().y);
    harness.hover_at(pos);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
    harness
        .query_all_by_role_and_label(Role::Button, "Move group")
        .find(|node| node.rect().width() > 20.0)
        .expect("the context menu has Move group")
        .hover();
    harness.run();
}

fn tabs_of(space: &DockSpace<String>, dock: &str) -> Vec<String> {
    space
        .dock(dock)
        .unwrap()
        .iter()
        .filter_map(|node| node.tabs().map(<[String]>::to_vec))
        .flatten()
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_menu_names_docks_by_the_viewers_titles() {
    let mut harness = harness();
    open_move_group_menu(&mut harness);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "start_side")
            .is_none(),
        "a titled dock is not shown by its name"
    );
    harness
        .get_by_role_and_label(Role::Button, "Left panel")
        .click();
    harness.run();
    assert_eq!(tabs_of(harness.state(), "start_side"), ["Files", "Editor"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dock_without_a_title_keeps_its_name() {
    let mut harness = harness();
    open_move_group_menu(&mut harness);
    harness
        .get_by_role_and_label(Role::Button, "end_side")
        .click();
    harness.run();
    assert_eq!(tabs_of(harness.state(), "end_side"), ["Editor"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_of_the_spaces_making_is_named_by_its_active_tab() {
    let mut space = space();
    let float = float_out(&mut space, "spare", &["Notes", "Help"]);
    assert_eq!(float, "float-1");
    let mut harness = harness_of(space);
    open_move_group_menu(&mut harness);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "float-1")
            .is_none(),
        "the space's own name for the float is not shown"
    );
    harness.get_by_role_and_label(Role::Button, "Notes");
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Help")
            .is_none()
    );
    // A dock the application named keeps its name.
    harness.get_by_role_and_label(Role::Button, "spare");

    // The float's active tab changes: so does its item.
    let surface = harness.state().dock_surface("float-1").unwrap();
    harness
        .state_mut()
        .state_mut()
        .set_active_tab(TabPath::new(surface, NodeIndex::root(), TabIndex(1)))
        .unwrap();
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Notes")
            .is_none()
    );
    harness.get_by_role_and_label(Role::Button, "Help").click();
    harness.run();
    assert_eq!(
        tabs_of(harness.state(), "float-1"),
        ["Notes", "Help", "Editor"]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_viewers_title_wins_over_the_floats_window_title() {
    let mut space = space();
    float_out(&mut space, "spare", &["Notes"]);
    let titled = float_out(&mut space, "other", &["Log"]);
    assert_eq!(titled, "float-2");
    let mut harness = harness_of(space);
    open_move_group_menu(&mut harness);
    harness.get_by_role_and_label(Role::Button, "Notes");
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Log")
            .is_none(),
        "the viewer's title replaces the window title"
    );
    harness
        .get_by_role_and_label(Role::Button, "Scratch")
        .click();
    harness.run();
    assert_eq!(tabs_of(harness.state(), "float-2"), ["Log", "Editor"]);
}
