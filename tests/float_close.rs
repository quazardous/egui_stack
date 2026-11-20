//! Closing a float over the main window that has no title bar of its own, on the
//! multi-viewport harness (`support/multi.rs`): its top tab bar has a "Close window" button
//! beside the move handle, which asks `TabViewer::close_policy` as the title bar's close button
//! does, whether or not its tabs can be closed. It is the bar's only close button while the
//! float holds one tab group: the group's close all button would close the window too.
//! With `FloatsArea::show_add_buttons`, the bar keeps its group's add button, as a docked
//! group's bar does, and a click on it reaches `TabViewer::on_add` for the float's group.

#[path = "support/multi.rs"]
mod multi;

use egui::{Id, Pos2, Rect, Ui, UiBuilder, ViewportId, WidgetText, pos2};
use egui_stack::layout::{Content, Dir, FloatHost, Layout, Zone};
use egui_stack::{
    ClosePolicy, DockArea, DockRules, DockSpace, Docked, FloatClose, FloatEvent, NodePath,
    TabViewer, ViewportHost,
};
use multi::Multi;

const MAIN: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(300.0, 300.0));
const SIDE: Rect = Rect::from_min_max(pos2(300.0, 0.0), pos2(600.0, 300.0));
const OUTSIDE: Pos2 = pos2(320.0, 420.0);
const CLOSE: &str = "Close window";

/// Tabs that cannot be closed, and the policy a float's close gets.
struct Viewer {
    tab_rects: Vec<(String, Rect)>,
    closeable: bool,
    policy: ClosePolicy,
    asked: usize,
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
        ui.label(tab.as_str());
    }

    fn is_closeable(&self, _tab: &String) -> bool {
        self.closeable
    }

    fn close_policy(&mut self, _close: &FloatClose) -> ClosePolicy {
        self.asked += 1;
        self.policy.clone()
    }

    fn on_tab_button(&mut self, tab: &mut String, response: &egui::Response) {
        self.tab_rects.push((tab.clone(), response.rect));
    }

    fn on_add(&mut self, path: NodePath) {
        self.added.push(path);
    }
}

struct Host;

impl ViewportHost for Host {}

struct App {
    docked: Docked<String>,
    viewer: Viewer,
    title_bar: bool,
    /// `DockArea::show_leaf_close_all_buttons` of the floats' areas.
    close_all: bool,
    /// `FloatsArea::show_add_buttons`.
    add: bool,
}

fn app(policy: ClosePolicy) -> App {
    let mut space = DockSpace::new().with_float_host(FloatHost::InWindow);
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
        viewer: Viewer {
            tab_rects: Vec::new(),
            closeable: false,
            policy,
            asked: 0,
            added: Vec::new(),
        },
        title_bar: false,
        close_all: true,
        add: false,
    }
}

fn frame(app: &mut App) -> impl FnMut(&mut Ui) + '_ {
    move |ui| {
        if ui.ctx().viewport_id() == ViewportId::ROOT {
            app.viewer.tab_rects.clear();
        }
        for (name, rect) in [("main", MAIN), ("side", SIDE)] {
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                DockArea::dock(&mut app.docked.space, name).show_inside(ui, &mut app.viewer);
            });
        }
        let mut host = Host;
        let close_all = app.close_all;
        app.docked
            .floats()
            .host(&mut host)
            .title_bar(app.title_bar)
            .show_add_buttons(app.add)
            .configure(move |_float, area| area.show_leaf_close_all_buttons(close_all))
            .show(ui.ctx(), &mut app.viewer);
    }
}

/// Starts the application and drags `B` out of every dock: its float, `float-1`.
fn detach_b(multi: &mut Multi, app: &mut App) {
    multi.passes(3, frame(app));
    let b = tab_rect(app, "B").expect("B is shown").center();
    multi.drag(b, OUTSIDE, frame(app));
    multi.passes(3, frame(app));
    assert!(app.docked.layout.float("float-1").is_some(), "B floats");
    app.docked.space.take_float_events();
}

fn tab_rect(app: &App, name: &str) -> Option<Rect> {
    app.viewer
        .tab_rects
        .iter()
        .find(|(tab, _)| tab == name)
        .map(|(_, rect)| *rect)
}

/// Clicks the float's "Close window" button, found by its accessible name.
fn click_close(multi: &mut Multi, app: &mut App) {
    let close = multi
        .find(ViewportId::ROOT, CLOSE)
        .expect("the float's close button has an accessible name")
        .center();
    multi.press(ViewportId::ROOT, close, frame(app));
    multi.release(ViewportId::ROOT, close, frame(app));
    multi.passes(3, frame(app));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_whose_tabs_cannot_close_closes_by_its_close_window_button() {
    let mut multi = Multi::new();
    let mut app = app(ClosePolicy::ReturnToOrigin);
    detach_b(&mut multi, &mut app);
    let handle = multi
        .find(ViewportId::ROOT, "Move window")
        .expect("the move handle is shown");
    let close = multi
        .find(ViewportId::ROOT, CLOSE)
        .expect("the close button is shown");
    assert!(
        (close.center().y - handle.center().y).abs() < 1.0 && close.left() >= handle.right() - 1.0,
        "the close button {close:?} is beside the move handle {handle:?}"
    );

    click_close(&mut multi, &mut app);
    assert_eq!(app.viewer.asked, 1, "close_policy is asked");
    assert!(
        app.docked.layout.float("float-1").is_none(),
        "the float closed"
    );
    let b = tab_rect(&app, "B").expect("B is shown again");
    assert!(MAIN.contains_rect(b), "B is back in its dock: {b:?}");
    let events = app.docked.space.take_float_events();
    assert!(
        events.iter().any(|event| matches!(
            event,
            FloatEvent::Closed { key, policy: Some(ClosePolicy::ReturnToOrigin), .. }
                if key == "float-1"
        )),
        "closed with the policy asked: {events:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_close_window_button_honours_a_keep_policy() {
    let mut multi = Multi::new();
    let mut app = app(ClosePolicy::Keep);
    detach_b(&mut multi, &mut app);
    click_close(&mut multi, &mut app);
    assert_eq!(app.viewer.asked, 1, "close_policy is asked");
    assert!(
        app.docked.layout.float("float-1").is_some(),
        "the float stays open"
    );
    let b = tab_rect(&app, "B").expect("B is shown");
    assert!(!MAIN.contains_rect(b), "B is still in its float: {b:?}");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_with_its_title_bar_has_no_close_window_button_on_its_tab_bar() {
    let mut multi = Multi::new();
    let mut app = app(ClosePolicy::ReturnToOrigin);
    app.title_bar = true;
    detach_b(&mut multi, &mut app);
    let handle = multi
        .find(ViewportId::ROOT, "Move window")
        .expect("the move handle is shown");
    // egui's own close button of the title bar has the same name, above the tab bar.
    let close = multi
        .find(ViewportId::ROOT, CLOSE)
        .expect("the title bar's close button");
    assert!(
        close.bottom() <= handle.top(),
        "the only close button {close:?} is the title bar's, above the tab bar {handle:?}"
    );
}

/// The close buttons on the float's top bar, the row of its move handle, left to right: the
/// float's close window button and its groups' close all buttons.
fn close_buttons_on_top_bar(multi: &Multi) -> Vec<(String, Rect)> {
    let handle = multi
        .find(ViewportId::ROOT, "Move window")
        .expect("the move handle is shown");
    let mut closes: Vec<_> = multi
        .labels(ViewportId::ROOT)
        .into_iter()
        .filter(|(label, rect)| {
            (label == CLOSE || label == "Close all tabs")
                && (rect.center().y - handle.center().y).abs() < 1.0
        })
        .collect();
    closes.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
    closes
}

/// A float holding one tab group shows one close button on its top bar, its close window
/// button beside the move handle, whatever `show_leaf_close_all_buttons` and whether or not
/// its tabs can be closed: the group's close all button would close the window too, a second
/// cross beside the first.
#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_of_one_group_has_one_close_button_on_its_top_bar() {
    for close_all in [true, false] {
        for closeable in [false, true] {
            let mut multi = Multi::new();
            let mut app = app(ClosePolicy::ReturnToOrigin);
            app.close_all = close_all;
            app.viewer.closeable = closeable;
            detach_b(&mut multi, &mut app);
            let closes = close_buttons_on_top_bar(&multi);
            let names: Vec<_> = closes.iter().map(|(label, _)| label.as_str()).collect();
            assert_eq!(
                names,
                [CLOSE],
                "close_all {close_all}, closeable {closeable}: one close button on the top bar"
            );
            let handle = multi.find(ViewportId::ROOT, "Move window").unwrap();
            assert!(
                closes[0].1.left() >= handle.right() - 1.0,
                "it is right of the move handle {handle:?}: {closes:?}"
            );
        }
    }
}

/// The float's top bar, its widgets left to right relative to the bar's left, against
/// `tests/painting/float_top_bar.txt` (`UPDATE_SNAPSHOTS=1` writes it again).
#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_top_bar_of_a_float_snapshot() {
    top_bar_snapshot(false, "float_top_bar.txt");
}

/// The same bar with add buttons on, against `tests/painting/float_top_bar_add.txt`: the
/// group's add button after its tab, its close window button still the bar's only cross.
#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_top_bar_of_a_float_with_add_buttons_snapshot() {
    top_bar_snapshot(true, "float_top_bar_add.txt");
}

fn top_bar_snapshot(add: bool, file: &str) {
    let mut multi = Multi::new();
    let mut app = app(ClosePolicy::ReturnToOrigin);
    app.viewer.closeable = true;
    app.add = add;
    detach_b(&mut multi, &mut app);
    let handle = multi.find(ViewportId::ROOT, "Move window").unwrap();
    let b = tab_rect(&app, "B").expect("B is shown");
    let mut row: Vec<_> = multi
        .labels(ViewportId::ROOT)
        .into_iter()
        .filter(|(_, rect)| {
            (rect.center().y - handle.center().y).abs() < 2.0
                && rect.left() >= b.left() - 1.0
                && rect.right() <= handle.right() + 40.0
        })
        .collect();
    row.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
    let mut text = String::new();
    for (label, rect) in row {
        text += &format!(
            "{label}: x {:.1}..{:.1}\n",
            rect.left() - b.left(),
            rect.right() - b.left()
        );
    }
    let path = format!("{}/tests/painting/{file}", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{path}: {e}; run with UPDATE_SNAPSHOTS=1 to write it"));
    assert_eq!(text, expected, "the float's top bar changed");
}

/// With add buttons on, a float's tab group shows its add button on the float's top bar, as a
/// docked group does, left of the move handle and with one close button still; a click on it
/// calls `TabViewer::on_add` with the float's group. Off, the bar has none.
#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_with_add_buttons_shows_one_add_button_that_reaches_on_add() {
    for add in [false, true] {
        let mut multi = Multi::new();
        let mut app = app(ClosePolicy::ReturnToOrigin);
        app.add = add;
        detach_b(&mut multi, &mut app);
        let handle = multi
            .find(ViewportId::ROOT, "Move window")
            .expect("the move handle is shown");
        let pluses: Vec<_> = multi
            .labels(ViewportId::ROOT)
            .into_iter()
            .filter(|(label, rect)| {
                label == "Add tab" && (rect.center().y - handle.center().y).abs() < 1.0
            })
            .map(|(_, rect)| rect)
            .collect();
        assert_eq!(
            pluses.len(),
            usize::from(add),
            "add buttons {add}: add buttons on the float's top bar: {pluses:?}"
        );
        let names: Vec<_> = close_buttons_on_top_bar(&multi)
            .into_iter()
            .map(|(label, _)| label)
            .collect();
        assert_eq!(names, [CLOSE], "add buttons {add}: one close button");
        if !add {
            continue;
        }
        let plus = pluses[0];
        let b = tab_rect(&app, "B").expect("B is shown");
        assert!(
            plus.left() >= b.right() - 1.0 && plus.right() <= handle.left() + 1.0,
            "the add button {plus:?} is between the tab {b:?} and the move handle {handle:?}"
        );

        multi.press(ViewportId::ROOT, plus.center(), frame(&mut app));
        multi.release(ViewportId::ROOT, plus.center(), frame(&mut app));
        multi.passes(2, frame(&mut app));
        let dock = app
            .docked
            .layout
            .float("float-1")
            .and_then(|float| float.dock_name())
            .expect("the float shows a dock")
            .to_owned();
        let surface = app.docked.space.dock_surface(&dock);
        assert_eq!(app.viewer.added.len(), 1, "on_add is called once");
        assert_eq!(
            Some(app.viewer.added[0].surface),
            surface,
            "on_add gets the float's group: {:?}",
            app.viewer.added
        );
    }
}
