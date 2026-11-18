//! The handles that move a whole tab group (`DockArea::group_handles`): the empty part of the
//! tab bar, the grip at its end, a tab dragged with the group modifier held, and the Move group
//! menu. Each moves the whole group, its tabs in order and its active tab kept; each one turned
//! off does nothing; the viewer's veto and Escape keep the layout.

use egui::{
    Event, Id, Key, Modifiers, PointerButton, Pos2, Rect, Ui, UiBuilder, WidgetText,
    accesskit::Role,
};
use egui::{pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{
    DockArea, DockState, GroupHandles, NodeIndex, NodePath, Style, SurfaceIndex, TabDestination,
    TabIndex, TabViewer, tab_viewer::DropVerdict,
};

struct Viewer {
    refuse: bool,
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

    fn drop_allowed(&self, tab: &String, _destination: &TabDestination) -> DropVerdict {
        if self.refuse && tab == "C" {
            DropVerdict::Refuse(Some("C stays".into()))
        } else {
            DropVerdict::Allow
        }
    }
}

#[derive(Clone, Copy)]
struct Options {
    handles: GroupHandles,
    /// Tabs fill their bar: no empty part is left.
    full_bar: bool,
    refuse: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            handles: GroupHandles::ALL,
            full_bar: false,
            refuse: false,
        }
    }
}

struct App {
    dock_state: DockState<String>,
    options: Options,
}

const ID: &str = "dock";

/// `A` on the left; `B`, `C`, `D` on the right, `C` shown. The dock takes the left 600 points
/// of an 800 × 400 screen.
fn harness(options: Options) -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec!["A".to_owned()]);
    let [_, right] = dock_state.main_surface_mut().split_right(
        NodeIndex::root(),
        0.5,
        vec!["B".to_owned(), "C".to_owned(), "D".to_owned()],
    );
    dock_state
        .leaf_mut(NodePath::new(SurfaceIndex::main(), right))
        .unwrap()
        .active = TabIndex(1);
    let app = App {
        dock_state,
        options,
    };
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            let dock = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0));
            let mut style = Style::from_egui(ui.style());
            style.tab_bar.fill_tab_bar = app.options.full_bar;
            let mut viewer = Viewer {
                refuse: app.options.refuse,
            };
            ui.scope_builder(UiBuilder::new().max_rect(dock), |ui| {
                DockArea::new(&mut app.dock_state)
                    .id(Id::unique(ID))
                    .style(style)
                    .group_handles(app.options.handles)
                    .show_inside(ui, &mut viewer);
            });
        },
        app,
    );
    harness.run();
    harness
}

fn leaf_rect(harness: &Harness<'_, App>, tab: &str) -> Rect {
    let surface = harness.state().dock_state.main_surface();
    let (node, _) = surface.find_tab(&tab.to_owned()).unwrap();
    surface[node].rect().unwrap()
}

/// The leaves of the main surface with their active tab.
fn leaves(app: &App) -> Vec<(Vec<String>, usize)> {
    app.dock_state
        .main_surface()
        .iter()
        .filter_map(|node| node.get_leaf())
        .map(|leaf| (leaf.tabs.clone(), leaf.active.0))
        .collect()
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

/// The group moved onto `A`'s leaf.
fn joined() -> Vec<(Vec<String>, usize)> {
    vec![(names(&["A", "B", "C", "D"]), 2)]
}

/// The grip of the right leaf's tab bar, if it is there.
fn right_grip(harness: &Harness<'_, App>) -> Option<Pos2> {
    harness
        .query_all_by_role_and_label(Role::Button, "Move group")
        .map(|node| node.rect().center())
        .find(|center| center.x > 300.0)
}

/// The empty part of the right leaf's tab bar, left of its grip and close-all button.
fn right_empty_space(harness: &Harness<'_, App>) -> Pos2 {
    let leaf = leaf_rect(harness, "B");
    pos2(leaf.right() - 60.0, leaf.top() + 10.0)
}

/// Presses at `start` (with `modifiers` held), moves down then to the middle of `A`'s leaf;
/// with `escape`, presses Escape there; then lets go.
fn drag_onto_a(harness: &mut Harness<'_, App>, start: Pos2, modifiers: Modifiers, escape: bool) {
    let leaf = leaf_rect(harness, "A");
    let target = pos2(leaf.center().x, leaf.center().y + 30.0);
    harness.event(Event::ModifiersChanged(modifiers));
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
    if escape {
        harness.key_press(Key::Escape);
        harness.step();
    }
    harness.drop_at(target);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run_steps(5);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_grip_moves_the_group() {
    let mut harness = harness(Options::default());
    let grip = right_grip(&harness).expect("the right tab bar has a grip");
    drag_onto_a(&mut harness, grip, Modifiers::NONE, false);
    assert_eq!(leaves(harness.state()), joined());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_full_tab_bar_is_still_moved_by_its_grip() {
    let mut harness = harness(Options {
        full_bar: true,
        ..Options::default()
    });
    // The tabs fill the bar up to the grip: no empty part is left to grab.
    let c = harness.get_by_role_and_label(Role::Tab, "D").rect();
    let grip = right_grip(&harness).expect("the right tab bar has a grip");
    assert!(grip.x - c.right() < 16.0, "{c:?} {grip:?}");
    drag_onto_a(&mut harness, grip, Modifiers::NONE, false);
    assert_eq!(leaves(harness.state()), joined());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn without_its_flag_the_grip_is_gone() {
    let harness = harness(Options {
        handles: GroupHandles::ALL.with(GroupHandles::GRIP, false),
        ..Options::default()
    });
    assert_eq!(right_grip(&harness), None);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_empty_space_moves_the_group() {
    let mut harness = harness(Options::default());
    let start = right_empty_space(&harness);
    drag_onto_a(&mut harness, start, Modifiers::NONE, false);
    assert_eq!(leaves(harness.state()), joined());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn without_its_flag_the_empty_space_moves_nothing() {
    let mut harness = harness(Options {
        handles: GroupHandles::ALL.with(GroupHandles::EMPTY_SPACE, false),
        ..Options::default()
    });
    let before = leaves(harness.state());
    let start = right_empty_space(&harness);
    drag_onto_a(&mut harness, start, Modifiers::NONE, false);
    assert_eq!(leaves(harness.state()), before);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_dragged_with_shift_carries_its_group() {
    let mut harness = harness(Options::default());
    let start = harness
        .get_by_role_and_label(Role::Tab, "C")
        .rect()
        .center();
    drag_onto_a(&mut harness, start, Modifiers::SHIFT, false);
    assert_eq!(leaves(harness.state()), joined());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn without_its_flag_shift_drags_the_tab_alone() {
    let mut harness = harness(Options {
        handles: GroupHandles::ALL.with(GroupHandles::MODIFIER_DRAG, false),
        ..Options::default()
    });
    let start = harness
        .get_by_role_and_label(Role::Tab, "C")
        .rect()
        .center();
    drag_onto_a(&mut harness, start, Modifiers::SHIFT, false);
    let all = leaves(harness.state());
    assert!(all.contains(&(names(&["A", "C"]), 1)), "{all:?}");
    assert!(
        all.iter().any(|(tabs, _)| *tabs == names(&["B", "D"])),
        "{all:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_plain_tab_drag_still_moves_the_tab_alone() {
    let mut harness = harness(Options::default());
    let start = harness
        .get_by_role_and_label(Role::Tab, "C")
        .rect()
        .center();
    drag_onto_a(&mut harness, start, Modifiers::NONE, false);
    let all = leaves(harness.state());
    assert!(all.contains(&(names(&["A", "C"]), 1)), "{all:?}");
}

/// Right-clicks `C`'s title, clear of its close button.
fn right_click_c(harness: &mut Harness<'_, App>) {
    let rect = harness.get_by_role_and_label(Role::Tab, "C").rect();
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
}

/// Opens `C`'s context menu, then its Move group submenu. Returns whether the submenu is there.
fn open_move_group_menu(harness: &mut Harness<'_, App>) -> bool {
    right_click_c(harness);
    let Some(menu) = harness
        .query_all_by_role_and_label(Role::Button, "Move group")
        .find(|node| node.rect().width() > 20.0)
    else {
        return false;
    };
    menu.hover();
    harness.run();
    true
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

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_menu_moves_the_group_to_a_new_window() {
    let mut harness = harness(Options::default());
    assert!(open_move_group_menu(&mut harness));
    harness
        .get_by_role_and_label(Role::Button, "New window")
        .click();
    harness.run();
    assert_eq!(leaves(harness.state()), [(names(&["A"]), 0)]);
    assert_eq!(windows(harness.state()), [names(&["B", "C", "D"])]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_grip_opens_the_menu_by_keyboard() {
    let mut harness = harness(Options::default());
    let grip = harness
        .query_all_by_role_and_label(Role::Button, "Move group")
        .find(|node| node.rect().center().x > 300.0)
        .unwrap();
    grip.focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "New window")
        .click();
    harness.run();
    assert_eq!(windows(harness.state()), [names(&["B", "C", "D"])]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn without_its_flag_the_menu_has_no_move_group() {
    let mut harness = harness(Options {
        handles: GroupHandles::ALL.with(GroupHandles::MENU, false),
        ..Options::default()
    });
    right_click_c(&mut harness);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Close")
            .is_some()
    );
    let in_menu = harness
        .query_all_by_role_and_label(Role::Button, "Move group")
        .filter(|node| node.rect().width() > 20.0)
        .count();
    assert_eq!(in_menu, 0, "only the grips are named Move group");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_vetoed_group_stays_put() {
    let mut harness = harness(Options {
        refuse: true,
        ..Options::default()
    });
    let before = leaves(harness.state());
    let grip = right_grip(&harness).unwrap();
    drag_onto_a(&mut harness, grip, Modifiers::NONE, false);
    assert_eq!(leaves(harness.state()), before);

    // The menu greys the place out rather than moving the group.
    assert!(open_move_group_menu(&mut harness));
    harness
        .get_by_role_and_label(Role::Button, "New window")
        .click();
    harness.run();
    assert_eq!(leaves(harness.state()), before);
    assert!(windows(harness.state()).is_empty());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn escape_cancels_a_group_drag() {
    let mut harness = harness(Options::default());
    let before = leaves(harness.state());
    let grip = right_grip(&harness).unwrap();
    drag_onto_a(&mut harness, grip, Modifiers::NONE, true);
    assert_eq!(leaves(harness.state()), before);
}
