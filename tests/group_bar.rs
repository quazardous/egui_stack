//! A tab group's bar: the opt-in Eject group button (`DockArea::show_leaf_eject_buttons`),
//! the application's own buttons (`TabViewer::group_bar_buttons`), and the group's menu on a
//! right click on the empty part of the bar (`DockArea::group_context_menus`), read through
//! the accessibility tree.

use egui::{
    Event, Id, Modifiers, PointerButton, Pos2, Rect, Ui, UiBuilder, WidgetText, accesskit::Role,
    pos2, vec2,
};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, NodeIndex, NodePath, SurfaceIndex, TabViewer};

#[derive(Default)]
struct Viewer {
    /// Keeps `C` out of windows.
    c_stays: bool,
    /// The application's buttons on each bar.
    buttons: Vec<String>,
    /// The paths of the bars whose application button was clicked.
    clicked: Vec<(NodePath, usize)>,
    /// Adds a "Rename group" item to the group's menu.
    group_item: bool,
    /// The paths `on_add` was called with.
    added: Vec<NodePath>,
}

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

    fn allowed_in_windows(&self, tab: &mut String) -> bool {
        !(self.c_stays && tab == "C")
    }

    fn group_bar_buttons(&mut self, _path: NodePath) -> Vec<String> {
        self.buttons.clone()
    }

    fn on_group_bar_button(
        &mut self,
        ui: &mut Ui,
        path: NodePath,
        index: usize,
        response: &egui::Response,
    ) {
        ui.painter()
            .circle_filled(response.rect.center(), 3.0, ui.visuals().text_color());
        if response.clicked() {
            self.clicked.push((path, index));
        }
    }

    fn group_context_menu(&mut self, ui: &mut Ui, _path: NodePath) {
        if self.group_item {
            let _ = ui.button("Rename group");
        }
    }

    fn on_add(&mut self, path: NodePath) {
        self.added.push(path);
    }
}

struct App {
    dock_state: DockState<String>,
    viewer: Viewer,
    eject: bool,
    add: bool,
}

impl App {
    fn new(dock_state: DockState<String>) -> Self {
        Self {
            dock_state,
            viewer: Viewer::default(),
            eject: false,
            add: false,
        }
    }
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

/// `A`, `B`, `C` on the left, `D` on the right.
fn harness(app: impl FnOnce(&mut App)) -> Harness<'static, App> {
    let mut dock_state = DockState::new(names(&["A", "B", "C"]));
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, names(&["D"]));
    let mut state = App::new(dock_state);
    app(&mut state);
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            let dock = Rect::from_min_size(Pos2::ZERO, vec2(800.0, 400.0));
            ui.scope_builder(UiBuilder::new().max_rect(dock), |ui| {
                DockArea::new(&mut app.dock_state)
                    .show_leaf_eject_buttons(app.eject)
                    .show_add_buttons(app.add)
                    .show_inside(ui, &mut app.viewer);
            });
        },
        state,
    );
    harness.run();
    harness
}

/// The tabs of each floating window.
fn windows(app: &App) -> Vec<Vec<String>> {
    app.dock_state
        .iter_surfaces()
        .skip(1)
        .filter_map(|surface| surface.node_tree())
        .map(|tree| {
            tree.iter()
                .filter_map(|node| node.get_leaf())
                .flat_map(|leaf| leaf.tabs.clone())
                .collect()
        })
        .collect()
}

fn count(harness: &Harness<'_, App>, label: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .count()
}

fn right_click(harness: &mut Harness<'_, App>, pos: Pos2) {
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
}

/// Right-clicks the empty part of the bar holding `tab`, right of its last tab.
fn right_click_bar_after(harness: &mut Harness<'_, App>, tab: &str) {
    let rect = harness.get_by_role_and_label(Role::Tab, tab).rect();
    right_click(harness, pos2(rect.right() + 40.0, rect.center().y));
}

fn menu_items(harness: &Harness<'_, App>) -> Vec<&'static str> {
    [
        "Rename group",
        "Move group",
        "Eject group",
        "Add tab",
        "Close all tabs",
        "Eject",
        "Close",
    ]
    .into_iter()
    .filter(|item| {
        // A menu item, not a bar button of the same name (those are 24 points wide at most).
        harness
            .query_all_by_role_and_label(Role::Button, item)
            .any(|node| node.rect().width() > 30.0)
    })
    .collect()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn no_eject_button_by_default() {
    let harness = harness(|_| {});
    assert_eq!(count(&harness, "Eject group"), 0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_eject_button_floats_the_group_with_its_tabs_in_order() {
    let mut harness = harness(|app| app.eject = true);
    assert_eq!(count(&harness, "Eject group"), 2, "one per group");
    let left = harness.get_by_role_and_label(Role::Tab, "A").rect().left();
    harness
        .query_all_by_role_and_label(Role::Button, "Eject group")
        .min_by(|a, b| {
            (a.rect().left() - left)
                .abs()
                .total_cmp(&(b.rect().left() - left).abs())
        })
        .filter(|node| node.rect().left() < 400.0)
        .expect("the left group's button")
        .click();
    harness.run();
    harness.run();
    assert_eq!(windows(harness.state()), [names(&["A", "B", "C"])]);
    // The window holds the group alone: its bar has no eject button; D's still has one.
    assert_eq!(count(&harness, "Eject group"), 1);
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, "Eject group")
            .all(|node| node.rect().left() > 400.0),
        "only D's bar keeps its button"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_group_with_a_tab_kept_out_of_windows_has_no_eject_button() {
    let harness = harness(|app| {
        app.eject = true;
        app.viewer.c_stays = true;
    });
    assert_eq!(count(&harness, "Eject group"), 1, "D's only");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_application_s_bar_buttons_are_named_buttons() {
    let mut harness = harness(|app| app.viewer.buttons = names(&["Pin group"]));
    assert_eq!(count(&harness, "Pin group"), 2, "one per group");
    let node = harness
        .query_all_by_role_and_label(Role::Button, "Pin group")
        .find(|node| node.rect().left() > 400.0)
        .expect("D's bar has one");
    node.click();
    harness.run();
    let clicked = &harness.state().viewer.clicked;
    assert_eq!(clicked.len(), 1, "{clicked:?}");
    assert_eq!(clicked[0].1, 0);
    let surface = harness.state().dock_state.main_surface();
    let (d, _) = surface.find_tab(&"D".to_owned()).unwrap();
    assert_eq!(clicked[0].0, NodePath::new(SurfaceIndex::main(), d));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_right_click_on_the_empty_bar_opens_the_group_s_menu() {
    let mut harness = harness(|app| {
        app.add = true;
        app.viewer.group_item = true;
    });
    right_click_bar_after(&mut harness, "C");
    assert_eq!(
        menu_items(&harness),
        [
            "Rename group",
            "Move group",
            "Eject group",
            "Add tab",
            "Close all tabs"
        ]
    );

    harness
        .query_all_by_role_and_label(Role::Button, "Add tab")
        .find(|node| node.rect().width() > 30.0)
        .unwrap()
        .click();
    harness.run();
    assert_eq!(harness.state().viewer.added.len(), 1);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_group_s_menu_ejects_the_group() {
    let mut harness = harness(|_| {});
    right_click_bar_after(&mut harness, "C");
    assert_eq!(
        menu_items(&harness),
        ["Move group", "Eject group", "Close all tabs"],
        "no Add tab without add buttons"
    );
    harness
        .get_by_role_and_label(Role::Button, "Eject group")
        .click();
    harness.run();
    harness.run();
    assert_eq!(windows(harness.state()), [names(&["A", "B", "C"])]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_right_click_on_a_tab_still_opens_the_tab_s_menu() {
    let mut harness = harness(|app| app.viewer.group_item = true);
    let rect = harness.get_by_role_and_label(Role::Tab, "B").rect();
    right_click(&mut harness, pos2(rect.left() + 6.0, rect.center().y));
    assert_eq!(menu_items(&harness), ["Move group", "Eject", "Close"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_group_s_menu_can_be_turned_off() {
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            DockArea::new(&mut app.dock_state)
                .group_context_menus(false)
                .show_inside(ui, &mut app.viewer);
        },
        App::new(DockState::new(names(&["A"]))),
    );
    harness.run();
    right_click_bar_after(&mut harness, "A");
    assert!(menu_items(&harness).is_empty());
}
