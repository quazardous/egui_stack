//! Moving a float's window (`FloatsArea::float_move`, `GroupHandles::MOVE_WINDOW`), on the
//! multi-viewport harness (`support/multi.rs`): the move handle moves the window (the system's
//! move on a native float, the `egui::Window` over the main window) with the move cursor;
//! by default a move never docks; with `FloatMove::Dock` a window released over a dock docks
//! whole, unless refused or where positions are hidden.

#[path = "support/multi.rs"]
mod multi;

use egui::{
    CursorIcon, Event, Id, Pos2, Rect, Ui, UiBuilder, ViewportCommand, ViewportId, WidgetText,
    pos2, vec2,
};
use egui_stack::layout::{Content, Dir, FloatHost, Layout, Screen, Zone};
use egui_stack::tab_viewer::{DockDrop, DropVerdict};
use egui_stack::{
    DockArea, DockRules, DockSpace, Docked, FloatMove, GroupHandles, TabViewer, ViewportHost,
};
use multi::Multi;

const MAIN: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(300.0, 300.0));
const SIDE: Rect = Rect::from_min_max(pos2(300.0, 0.0), pos2(600.0, 300.0));
const OUTSIDE: Pos2 = pos2(320.0, 420.0);
const LABEL: &str = "Move window";

#[derive(Default)]
struct Viewer {
    tab_rects: Vec<(String, Rect)>,
    /// Refuses every drop into the main dock.
    refuse_main: bool,
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
        ui.label(tab.as_str());
    }

    fn on_tab_button(&mut self, tab: &mut String, response: &egui::Response) {
        self.tab_rects.push((tab.clone(), response.rect));
    }

    fn dock_drop_allowed(&self, _tab: &String, drop: &DockDrop<'_>) -> DropVerdict {
        if self.refuse_main && drop.to == Some("main") {
            DropVerdict::Refuse(None)
        } else {
            DropVerdict::Allow
        }
    }
}

struct Host;

impl ViewportHost for Host {
    fn screens(&self) -> Vec<Screen> {
        vec![Screen::new(
            "one",
            Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0)),
            1.0,
        )]
    }
}

struct App {
    docked: Docked<String>,
    viewer: Viewer,
    handles: GroupHandles,
    float_move: FloatMove,
}

fn app(host: FloatHost, float_move: FloatMove) -> App {
    let mut space = DockSpace::new().with_float_host(host);
    let tabs = |names: &[&str]| names.iter().map(|n| (*n).to_owned()).collect();
    space.add_dock("main", DockRules::default(), tabs(&["A", "B", "C"]));
    space.add_dock("side", DockRules::default(), tabs(&["S"]));
    let layout = Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::leaf("main", Content::Dock("main".into())),
            Zone::leaf("side", Content::Dock("side".into())),
        ],
    ));
    App {
        docked: Docked::new(layout, space),
        viewer: Viewer::default(),
        handles: GroupHandles::ALL,
        float_move,
    }
}

fn frame(app: &mut App) -> impl FnMut(&mut Ui) + '_ {
    move |ui| {
        app.viewer.tab_rects.clear();
        for (name, rect) in [("main", MAIN), ("side", SIDE)] {
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                DockArea::dock(&mut app.docked.space, name).show_inside(ui, &mut app.viewer);
            });
        }
        app.docked
            .floats()
            .host(&mut Host)
            .group_handles(app.handles)
            .float_move(app.float_move)
            .show(ui.ctx(), &mut app.viewer);
    }
}

fn tabs(app: &App, dock: &str) -> Vec<String> {
    app.docked
        .space
        .dock(dock)
        .map(|tree| {
            tree.iter()
                .filter_map(|n| n.tabs())
                .flatten()
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn float_viewport() -> ViewportId {
    Host.viewport_id("float-1")
}

/// Starts the application and drags `B` out of every dock: its float, `float-1`.
fn detach_b(multi: &mut Multi, app: &mut App) {
    multi.passes(3, frame(app));
    let b = app
        .viewer
        .tab_rects
        .iter()
        .find(|(name, _)| name == "B")
        .map(|(_, rect)| rect.center())
        .expect("B is shown");
    multi.drag(b, OUTSIDE, frame(app));
    multi.passes(3, frame(app));
    assert!(app.docked.layout.float("float-1").is_some(), "B floats");
}

/// The move handle of the float, in its window's points.
fn handle(multi: &Multi, viewport: ViewportId) -> Pos2 {
    multi
        .find(viewport, LABEL)
        .unwrap_or_else(|| panic!("the float has a move handle"))
        .center()
}

/// Drags in a native float's window from `from` (its points) to `to` on the desktop, the
/// pointer given in the window's points as the system tells it while the window moves.
fn drag_native_to(multi: &mut Multi, app: &mut App, from: Pos2, to: Pos2) {
    let viewport = float_viewport();
    multi.press(viewport, from, frame(app));
    let start = multi.inner_rect(viewport).unwrap().min + from.to_vec2();
    for step in 1..=12 {
        let at = start + (to - start) * (step as f32 / 12.0);
        let origin = multi.inner_rect(viewport).map_or(Pos2::ZERO, |r| r.min);
        multi.queue(viewport, Event::PointerMoved(at - origin.to_vec2()));
        multi.pass(frame(app));
    }
    let origin = multi.inner_rect(viewport).map_or(Pos2::ZERO, |r| r.min);
    multi.release(viewport, to - origin.to_vec2(), frame(app));
    multi.passes(3, frame(app));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_native_float_s_move_handle_asks_the_system_to_move_it() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, FloatMove::default());
    detach_b(&mut multi, &mut app);
    let viewport = float_viewport();
    let at = handle(&multi, viewport);

    multi.queue(viewport, Event::PointerMoved(at));
    multi.pass(frame(&mut app));
    multi.pass(frame(&mut app));
    assert_eq!(multi.cursor(viewport), Some(CursorIcon::Move));

    multi.take_commands(viewport);
    multi.press(viewport, at, frame(&mut app));
    multi.move_held(viewport, at, at + vec2(30.0, 20.0), frame(&mut app));
    let commands = multi.take_commands(viewport);
    assert!(
        commands.contains(&ViewportCommand::StartDrag),
        "the system moves the window: {commands:?}"
    );
    multi.release(viewport, at + vec2(30.0, 20.0), frame(&mut app));
    assert_eq!(tabs(&app, "float-1"), ["B"], "a move never docks");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn by_default_moving_a_native_float_over_a_dock_never_docks_it() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, FloatMove::Move);
    detach_b(&mut multi, &mut app);
    let viewport = float_viewport();
    let at = handle(&multi, viewport);
    drag_native_to(&mut multi, &mut app, at, MAIN.center());
    assert!(app.docked.layout.float("float-1").is_some());
    assert_eq!(tabs(&app, "main"), ["A", "C"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn with_dock_a_native_float_moved_over_a_dock_docks_whole() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, FloatMove::Dock);
    detach_b(&mut multi, &mut app);
    let viewport = float_viewport();
    let before = multi.inner_rect(viewport).unwrap();
    let at = handle(&multi, viewport);
    multi.take_commands(viewport);

    // Half way, over nothing: the window followed the pointer, nothing docked.
    let viewport_origin = before.min;
    let midway = viewport_origin + at.to_vec2() + vec2(-40.0, -30.0);
    multi.press(viewport, at, frame(&mut app));
    for _ in 0..2 {
        // The system tells the pointer in the window's points, wherever the window now is.
        let origin = multi.inner_rect(viewport).unwrap().min;
        multi.queue(viewport, Event::PointerMoved(midway - origin.to_vec2()));
        multi.pass(frame(&mut app));
    }
    multi.pass(frame(&mut app));
    let commands = multi.take_commands(viewport);
    assert!(
        commands
            .iter()
            .any(|c| matches!(c, ViewportCommand::OuterPosition(_))),
        "the window moves itself: {commands:?}"
    );
    assert!(!commands.contains(&ViewportCommand::StartDrag));
    let moved = multi.inner_rect(viewport).unwrap();
    assert!(
        (moved.min - (before.min + vec2(-40.0, -30.0))).length() < 2.0,
        "{before:?} → {moved:?}"
    );
    let origin = multi.inner_rect(viewport).unwrap().min;
    multi.release(viewport, midway - origin.to_vec2(), frame(&mut app));
    multi.passes(2, frame(&mut app));
    assert!(app.docked.layout.float("float-1").is_some(), "over nothing");

    // Then over the main dock: B joins it, the float closes.
    let at = handle(&multi, viewport);
    drag_native_to(&mut multi, &mut app, at, MAIN.center());
    assert!(app.docked.layout.float("float-1").is_none());
    let mut main = tabs(&app, "main");
    main.sort();
    assert_eq!(main, ["A", "B", "C"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn with_dock_a_refused_dock_keeps_the_float() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, FloatMove::Dock);
    app.viewer.refuse_main = true;
    detach_b(&mut multi, &mut app);
    let at = handle(&multi, float_viewport());
    drag_native_to(&mut multi, &mut app, at, MAIN.center());
    assert!(app.docked.layout.float("float-1").is_some());
    assert_eq!(tabs(&app, "main"), ["A", "C"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn with_dock_hidden_positions_only_move() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, FloatMove::Dock);
    detach_b(&mut multi, &mut app);
    multi.hide_positions();
    multi.passes(2, frame(&mut app));
    let viewport = float_viewport();
    let at = handle(&multi, viewport);
    multi.take_commands(viewport);
    multi.press(viewport, at, frame(&mut app));
    multi.move_held(viewport, at, at + vec2(-200.0, -200.0), frame(&mut app));
    multi.release(viewport, at + vec2(-200.0, -200.0), frame(&mut app));
    multi.passes(2, frame(&mut app));
    let commands = multi.take_commands(viewport);
    assert!(
        commands.contains(&ViewportCommand::StartDrag),
        "{commands:?}"
    );
    assert!(app.docked.layout.float("float-1").is_some());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_move_handle_moves_a_float_over_the_main_window_and_never_docks_by_default() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, FloatMove::Move);
    detach_b(&mut multi, &mut app);
    let before = app.docked.layout.float("float-1").unwrap().place.rect;
    let at = handle(&multi, ViewportId::ROOT);

    multi.queue(ViewportId::ROOT, Event::PointerMoved(at));
    multi.pass(frame(&mut app));
    multi.pass(frame(&mut app));
    assert_eq!(multi.cursor(ViewportId::ROOT), Some(CursorIcon::Move));

    let to = at + vec2(-60.0, -40.0);
    multi.drag(at, to, frame(&mut app));
    let after = app.docked.layout.float("float-1").unwrap().place.rect;
    assert!(
        (after.min - (before.min + vec2(-60.0, -40.0))).length() < 2.0,
        "{before:?} → {after:?}"
    );

    // Over the main dock: it only moves.
    let at = handle(&multi, ViewportId::ROOT);
    multi.drag(at, MAIN.center(), frame(&mut app));
    assert!(app.docked.layout.float("float-1").is_some());
    assert_eq!(tabs(&app, "main"), ["A", "C"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn with_dock_a_float_over_the_main_window_docks_by_its_handle_or_title_bar() {
    // By the handle.
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, FloatMove::Dock);
    detach_b(&mut multi, &mut app);
    let at = handle(&multi, ViewportId::ROOT);
    multi.drag(at, MAIN.center(), frame(&mut app));
    assert!(app.docked.layout.float("float-1").is_none());
    let mut main = tabs(&app, "main");
    main.sort();
    assert_eq!(main, ["A", "B", "C"]);
    drop(multi);

    // By the title bar.
    let mut multi = Multi::new();
    let mut app = self::app(FloatHost::InWindow, FloatMove::Dock);
    detach_b(&mut multi, &mut app);
    let window = app.docked.layout.float("float-1").unwrap().place.rect;
    let title = window.left_top() + vec2(40.0, 8.0);
    multi.drag(title, SIDE.center(), frame(&mut app));
    assert!(app.docked.layout.float("float-1").is_none());
    let mut side = tabs(&app, "side");
    side.sort();
    assert_eq!(side, ["B", "S"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn by_default_the_title_bar_of_a_float_over_the_main_window_only_moves_it() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, FloatMove::default());
    detach_b(&mut multi, &mut app);
    let window = app.docked.layout.float("float-1").unwrap().place.rect;
    let title = window.left_top() + vec2(40.0, 8.0);
    multi.drag(title, SIDE.center(), frame(&mut app));
    let after = app
        .docked
        .layout
        .float("float-1")
        .expect("still a float")
        .place
        .rect;
    assert_ne!(after.min, window.min, "moved");
    assert_eq!(tabs(&app, "side"), ["S"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_move_handle_turned_off_is_not_there() {
    for host in [FloatHost::Native, FloatHost::InWindow] {
        let mut multi = Multi::new();
        let mut app = app(host, FloatMove::Dock);
        app.handles = GroupHandles::ALL.with(GroupHandles::MOVE_WINDOW, false);
        detach_b(&mut multi, &mut app);
        let viewport = match host {
            FloatHost::Native => float_viewport(),
            _ => ViewportId::ROOT,
        };
        assert!(multi.find(viewport, LABEL).is_none(), "{host:?}");
        // The group grip stays.
        assert!(multi.find(viewport, "Move group").is_some(), "{host:?}");
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn docks_outside_floats_have_no_move_handle() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, FloatMove::default());
    multi.passes(3, frame(&mut app));
    assert!(multi.find(ViewportId::ROOT, LABEL).is_none());
}
