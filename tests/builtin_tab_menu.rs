//! `DockArea::show_builtin_tab_menu(false)` leaves a tab's context menu to the viewer alone.

use egui::{Id, Ui, WidgetText, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, TabPath, TabViewer};

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

    fn context_menu(&mut self, ui: &mut Ui, _tab: &mut String, _path: TabPath) {
        let _ = ui.button("Rename");
    }
}

fn menu_items(builtin: bool) -> Vec<&'static str> {
    let mut harness = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        move |ui, dock_state: &mut DockState<String>| {
            DockArea::new(dock_state)
                .show_builtin_tab_menu(builtin)
                .hidable_tab_bars(true)
                .show_inside(ui, &mut Viewer);
        },
        DockState::new(vec!["First".to_owned(), "Second".to_owned()]),
    );
    harness.run();
    harness
        .get_by_role_and_label(Role::Tab, "First")
        .click_secondary();
    harness.run();

    ["Rename", "Eject", "Close", "Hide tab bar"]
        .into_iter()
        .filter(|item| {
            harness
                .query_by_role_and_label(Role::Button, item)
                .is_some()
        })
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_builtin_items_follow_the_viewers() {
    assert_eq!(
        menu_items(true),
        ["Rename", "Eject", "Close", "Hide tab bar"]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn without_the_builtin_items_the_viewers_stand_alone() {
    assert_eq!(menu_items(false), ["Rename"]);
}
