//! A float over the main window whose tab groups are all collapsed (their ▶ button) shrinks to
//! its tab bars, without its resize grip; expanded, it gets back the height it had. The height
//! it had is what its place keeps, so a layout saved collapsed and restored expands to it too.

use egui::{CursorIcon, Id, Pos2, Rect, Ui, WidgetText, accesskit::Role, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::layout::{Content, Layout, Zone};
use egui_stack::{DockRules, DockSpace, Docked, FloatShown, NodeIndex, TabIndex, TabViewer};

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

struct App {
    docked: Docked<String>,
    shown: Vec<FloatShown>,
}

/// The float's place: 300 × 240 points at (200, 80).
const PLACE: Rect = Rect::from_min_max(pos2(200.0, 80.0), pos2(500.0, 320.0));

/// `Editor`, `Console` and `Notes` floated out of the main dock, over the main window.
fn app() -> App {
    let mut space = DockSpace::new();
    space.add_dock(
        "main",
        DockRules::default(),
        vec![
            "Editor".to_owned(),
            "Console".to_owned(),
            "Notes".to_owned(),
        ],
    );
    let layout = Layout::new(Zone::leaf("main", Content::Dock("main".into())));
    let mut docked = Docked::new(layout, space);
    let main = docked.space.dock_surface("main").unwrap();
    docked
        .space
        .float_out_group(
            egui_stack::NodePath::new(main, NodeIndex::root()),
            PLACE,
            |tab| Id::unique(tab.as_str()),
        )
        .unwrap();
    App {
        docked,
        shown: Vec::new(),
    }
}

fn harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder().with_size([800.0, 600.0]).build_ui_state(
        |ui, app: &mut App| {
            app.shown.clear();
            let shown = &mut app.shown;
            app.docked
                .floats()
                .decorate(|_, float| shown.push(float.clone()))
                .show(ui.ctx(), &mut Viewer);
        },
        app,
    );
    harness.run();
    harness
}

/// The float as it was last drawn.
fn shown(harness: &Harness<'_, App>) -> FloatShown {
    harness
        .state()
        .shown
        .last()
        .cloned()
        .expect("a float is shown")
}

/// The float's dock: its only tree.
fn float_dock(harness: &Harness<'_, App>) -> String {
    shown(harness).dock
}

/// Clicks the collapse button of the float's group `index` (each group's toggles it).
fn toggle(harness: &mut Harness<'_, App>, index: usize) {
    harness
        .query_all_by_role_and_label(Role::Button, "Collapse")
        .nth(index)
        .expect("a collapse button")
        .click();
    harness.run();
}

/// Toggles every group of the float.
fn toggle_all(harness: &mut Harness<'_, App>) {
    let count = harness
        .query_all_by_role_and_label(Role::Button, "Collapse")
        .count();
    assert!(count > 0, "no collapse button");
    for index in 0..count {
        toggle(harness, index);
    }
}

/// The sum of the heights of the float's tab bars, stacked.
fn bars_height(harness: &Harness<'_, App>) -> f32 {
    shown(harness)
        .tab_bars
        .iter()
        .map(|(_, bar)| bar.height())
        .sum()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_collapsed_float_shrinks_to_its_tab_bar_and_expands_back() {
    let mut harness = harness(app());
    let before = shown(&harness).rect;
    assert!(before.height() > 150.0, "{before:?}");

    toggle_all(&mut harness);
    let collapsed = shown(&harness).rect;
    assert!(
        (collapsed.height() - bars_height(&harness)).abs() < 1.0,
        "the dock is as tall as its tab bar: {collapsed:?}, bars {}",
        bars_height(&harness)
    );
    assert_eq!(collapsed.min, before.min, "it stays where it was");
    assert!((collapsed.width() - before.width()).abs() < 1.0);

    toggle_all(&mut harness);
    harness.run();
    let after = shown(&harness).rect;
    assert!(
        (after.height() - before.height()).abs() < 1.0,
        "the height it had is back: {after:?}, was {before:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_collapsed_float_has_no_resize_grip() {
    let mut harness = harness(app());
    // The window's bottom-right corner, where egui's resize grip is.
    let corner = |harness: &Harness<'_, App>| -> Pos2 {
        let rect = shown(harness).rect;
        rect.right_bottom() + vec2(2.0, 2.0)
    };
    let cursor = |harness: &mut Harness<'_, App>, at: Pos2| {
        harness.hover_at(at);
        harness.run();
        harness.output().platform_output.cursor_icon
    };
    let at = corner(&harness);
    assert_eq!(cursor(&mut harness, at), CursorIcon::ResizeNwSe);

    toggle_all(&mut harness);
    let at = corner(&harness);
    assert_ne!(cursor(&mut harness, at), CursorIcon::ResizeNwSe);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_of_several_groups_shrinks_once_all_are_collapsed() {
    let mut app = app();
    // `Notes` below the other two, in the float.
    let dock = app.docked.space.dock_names().last().unwrap().to_owned();
    let tree = app.docked.space.dock_mut(&dock).unwrap();
    let notes = tree.remove_tab((NodeIndex::root(), TabIndex(2))).unwrap();
    tree.split_below(NodeIndex::root(), 0.5, vec![notes]);
    let mut harness = harness(app);
    assert_eq!(float_dock(&harness), dock);
    let before = shown(&harness).rect;

    // One group collapsed: the float keeps its height.
    toggle(&mut harness, 0);
    assert!((shown(&harness).rect.height() - before.height()).abs() < 1.0);

    // Both: as tall as the two bars and the separator between them.
    toggle(&mut harness, 1);
    let collapsed = shown(&harness).rect;
    let bars = shown(&harness).tab_bars;
    assert_eq!(bars.len(), 2);
    let span = bars[1].1.bottom() - bars[0].1.top();
    assert!(
        (collapsed.height() - span).abs() < 1.0,
        "{collapsed:?}, bars {bars:?}"
    );

    toggle_all(&mut harness);
    harness.run();
    assert!((shown(&harness).rect.height() - before.height()).abs() < 1.0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_restored_collapsed_expands_to_the_height_it_had() {
    let mut harness = harness(app());
    let before = shown(&harness).rect;
    let key = shown(&harness).key;
    let place = |harness: &Harness<'_, App>| {
        harness
            .state()
            .docked
            .layout
            .float(&key)
            .unwrap()
            .place
            .rect
    };
    let expanded = place(&harness);
    toggle_all(&mut harness);
    // Its place keeps the height to expand to: that is what is saved.
    assert_eq!(place(&harness).height(), expanded.height());

    // Restored in a new context: collapsed, then expanded to that height.
    let docked = std::mem::replace(&mut harness.state_mut().docked, app().docked);
    let mut harness = self::harness(App {
        docked,
        shown: Vec::new(),
    });
    harness.run();
    let collapsed = shown(&harness).rect;
    assert!(
        (collapsed.height() - bars_height(&harness)).abs() < 1.0,
        "{collapsed:?}"
    );
    toggle_all(&mut harness);
    harness.run();
    assert!(
        (shown(&harness).rect.height() - before.height()).abs() < 1.0,
        "{:?}, was {before:?}",
        shown(&harness).rect
    );
}

/// The float's group: its dock's root leaf.
fn float_group(harness: &Harness<'_, App>) -> egui_stack::NodePath {
    let dock = float_dock(harness);
    let surface = harness.state().docked.space.dock_surface(&dock).unwrap();
    egui_stack::NodePath::new(surface, NodeIndex::root())
}

/// Collapses or expands the float's group from code.
fn set_collapsed(harness: &mut Harness<'_, App>, collapsed: bool) {
    let path = float_group(harness);
    harness
        .state_mut()
        .docked
        .space
        .state_mut()
        .set_collapsed(path, collapsed)
        .expect("the group is a leaf");
    harness.run();
    harness.run();
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_collapsed_by_its_button_expands_from_code() {
    let mut harness = harness(app());
    let before = shown(&harness).rect;

    toggle_all(&mut harness);
    assert!(
        (shown(&harness).rect.height() - bars_height(&harness)).abs() < 1.0,
        "collapsed by its button"
    );

    set_collapsed(&mut harness, false);
    let after = shown(&harness).rect;
    assert!(
        (after.height() - before.height()).abs() < 1.0,
        "expanded from code, the height it had is back: {after:?}, was {before:?}"
    );
    let path = float_group(&harness);
    assert!(!harness.state().docked.space.state()[path].is_collapsed());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_collapses_and_expands_from_code_as_with_its_button() {
    let mut harness = harness(app());
    let before = shown(&harness).rect;

    set_collapsed(&mut harness, true);
    let collapsed = shown(&harness).rect;
    assert!(
        (collapsed.height() - bars_height(&harness)).abs() < 1.0,
        "collapsed from code, as tall as its tab bar: {collapsed:?}, bars {}",
        bars_height(&harness)
    );
    assert_eq!(collapsed.min, before.min, "it stays where it was");

    set_collapsed(&mut harness, false);
    let after = shown(&harness).rect;
    assert!(
        (after.height() - before.height()).abs() < 1.0,
        "expanded from code: {after:?}, was {before:?}"
    );

    // And the button still toggles it after.
    toggle_all(&mut harness);
    assert!((shown(&harness).rect.height() - bars_height(&harness)).abs() < 1.0);
}
