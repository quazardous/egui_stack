//! Ctrl+Tab and Ctrl+Shift+Tab cycle the active tab of the tab group under the pointer, next
//! and previous, wrapping around, as egui_tiles does: only that group changes, the AccessKit
//! tree follows, a focused text edit keeps the keys, `DockArea::cycle_tabs_with_keyboard(false)`
//! turns it off, and the floats' docks cycle too.

use egui::{Id, Key, Modifiers, Pos2, Rect, Ui, WidgetText, accesskit::Role, pos2};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::layout::{Content, Layout, Zone};
use egui_stack::{
    DockArea, DockRules, DockSpace, DockState, Docked, FloatShown, NodeIndex, NodePath, TabIndex,
    TabViewer,
};

struct Viewer {
    text: String,
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
        if tab == "Editor" {
            ui.text_edit_singleline(&mut self.text);
        } else {
            ui.label(format!("Body of {tab}"));
        }
    }
}

struct App {
    dock_state: DockState<String>,
    viewer: Viewer,
    cycle: bool,
}

/// Two groups side by side: `A1`, `A2`, `A3` on the left, `Editor`, `B2` on the right.
fn harness(cycle: bool) -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec!["A1".to_owned(), "A2".to_owned(), "A3".to_owned()]);
    dock_state.main_surface_mut().split_right(
        NodeIndex::root(),
        0.5,
        vec!["Editor".to_owned(), "B2".to_owned()],
    );
    let app = App {
        dock_state,
        viewer: Viewer {
            text: String::new(),
        },
        cycle,
    };
    let mut harness = Harness::builder()
        .with_size([800.0, 400.0])
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, app: &mut App| {
                DockArea::new(&mut app.dock_state)
                    .cycle_tabs_with_keyboard(app.cycle)
                    .show_inside(ui, &mut app.viewer);
            },
            app,
        );
    harness.run();
    harness
}

/// The active tab of the leaf `node` of the main surface.
fn active(harness: &Harness<'_, App>, node: usize) -> usize {
    let tree = harness.state().dock_state.main_surface();
    tree[NodeIndex(node)].get_leaf().expect("a leaf").active.0
}

/// The leaves' indices: the left one and the right one.
fn leaves(harness: &Harness<'_, App>) -> (usize, usize) {
    let tree = harness.state().dock_state.main_surface();
    let mut leaves: Vec<(f32, usize)> = tree
        .iter()
        .enumerate()
        .filter(|(_, node)| node.is_leaf())
        .map(|(index, node)| (node.rect().unwrap().center().x, index))
        .collect();
    leaves.sort_by(|a, b| a.0.total_cmp(&b.0));
    (leaves[0].1, leaves[1].1)
}

/// The middle of the body of the leaf `node`, below its tab bar.
fn over(harness: &Harness<'_, App>, node: usize) -> Pos2 {
    let rect = harness.state().dock_state.main_surface()[NodeIndex(node)]
        .rect()
        .unwrap();
    pos2(rect.center().x, rect.bottom() - 20.0)
}

fn ctrl_tab<S>(harness: &mut Harness<'_, S>) {
    harness.key_press_modifiers(Modifiers::CTRL, Key::Tab);
    harness.run();
}

fn ctrl_shift_tab<S>(harness: &mut Harness<'_, S>) {
    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab);
    harness.run();
}

fn selected<S>(harness: &Harness<'_, S>, title: &str) -> Option<bool> {
    harness
        .get_by_role_and_label(Role::Tab, title)
        .accesskit_node()
        .is_selected()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn ctrl_tab_cycles_forward_and_wraps_ctrl_shift_tab_goes_back() {
    let mut harness = harness(true);
    let (left, _) = leaves(&harness);
    harness.hover_at(over(&harness, left));
    harness.run();
    assert_eq!(active(&harness, left), 0);

    ctrl_tab(&mut harness);
    assert_eq!(active(&harness, left), 1);
    assert_eq!(selected(&harness, "A2"), Some(true), "AccessKit follows");
    assert_eq!(selected(&harness, "A1"), Some(false));
    ctrl_tab(&mut harness);
    assert_eq!(active(&harness, left), 2);
    ctrl_tab(&mut harness);
    assert_eq!(active(&harness, left), 0, "wraps to the first tab");

    ctrl_shift_tab(&mut harness);
    assert_eq!(active(&harness, left), 2, "wraps back to the last tab");
    assert_eq!(selected(&harness, "A3"), Some(true));
    ctrl_shift_tab(&mut harness);
    assert_eq!(active(&harness, left), 1);

    // The group cycled is the focused one, as after a click.
    let focused = harness.state().dock_state.focused_leaf();
    assert_eq!(focused.map(|path| path.node), Some(NodeIndex(left)));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn only_the_group_under_the_pointer_cycles() {
    let mut harness = harness(true);
    let (left, right) = leaves(&harness);
    harness.hover_at(over(&harness, right));
    harness.run();

    ctrl_tab(&mut harness);
    assert_eq!(active(&harness, right), 1);
    assert_eq!(active(&harness, left), 0, "the other group is left alone");
    assert_eq!(selected(&harness, "B2"), Some(true));
    assert_eq!(selected(&harness, "A1"), Some(true));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn nothing_cycles_when_turned_off() {
    let mut harness = harness(false);
    let (left, right) = leaves(&harness);
    harness.hover_at(over(&harness, left));
    harness.run();

    ctrl_tab(&mut harness);
    ctrl_shift_tab(&mut harness);
    assert_eq!((active(&harness, left), active(&harness, right)), (0, 0));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_focused_text_edit_keeps_ctrl_tab() {
    let mut harness = harness(true);
    let (_, right) = leaves(&harness);
    harness.get_by_role(Role::TextInput).click();
    harness.run();
    assert!(harness.ctx.text_edit_focused());
    harness.hover_at(over(&harness, right));
    harness.run();

    ctrl_tab(&mut harness);
    assert_eq!(active(&harness, right), 0, "the text edit kept the keys");

    // Once it lets go of the focus, the group cycles.
    harness
        .ctx
        .memory_mut(|m| m.surrender_focus(m.focused().unwrap()));
    harness.run();
    ctrl_tab(&mut harness);
    assert_eq!(active(&harness, right), 1);
}

struct FloatApp {
    docked: Docked<String>,
    viewer: Viewer,
    shown: Vec<FloatShown>,
}

/// `A1`, `A2` and `A3` floated out of the main dock, over the main window; the main dock is
/// not drawn.
#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_cycles_its_group() {
    let mut space = DockSpace::new();
    space.add_dock(
        "main",
        DockRules::default(),
        vec!["A1".to_owned(), "A2".to_owned(), "A3".to_owned()],
    );
    let layout = Layout::new(Zone::leaf("main", Content::Dock("main".into())));
    let mut docked = Docked::new(layout, space);
    let main = docked.space.dock_surface("main").unwrap();
    let place = Rect::from_min_max(pos2(200.0, 80.0), pos2(500.0, 320.0));
    docked
        .space
        .float_out_group(NodePath::new(main, NodeIndex::root()), place, |tab| {
            Id::unique(tab.as_str())
        })
        .unwrap();
    let app = FloatApp {
        docked,
        viewer: Viewer {
            text: String::new(),
        },
        shown: Vec::new(),
    };
    let mut harness = Harness::builder()
        .with_size([800.0, 600.0])
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, app: &mut FloatApp| {
                app.shown.clear();
                let shown = &mut app.shown;
                app.docked
                    .floats()
                    .decorate(|_, float| shown.push(float.clone()))
                    .show(ui.ctx(), &mut app.viewer);
            },
            app,
        );
    harness.run();

    let float = harness.state().shown.last().cloned().expect("a float");
    let active = |harness: &Harness<'_, FloatApp>| {
        let surface = harness
            .state()
            .docked
            .space
            .dock_surface(&float.dock)
            .unwrap();
        let tree = &harness.state().docked.space.state()[surface];
        let leaf = tree
            .iter()
            .find_map(|node| node.get_leaf())
            .expect("a leaf");
        leaf.active
    };
    harness.hover_at(float.rect.center());
    harness.run();
    assert_eq!(active(&harness), TabIndex(0));

    ctrl_tab(&mut harness);
    assert_eq!(active(&harness), TabIndex(1));
    assert_eq!(selected(&harness, "A2"), Some(true));
    ctrl_shift_tab(&mut harness);
    ctrl_shift_tab(&mut harness);
    assert_eq!(active(&harness), TabIndex(2), "wraps back to the last tab");
}
