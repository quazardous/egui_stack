//! Closing a float over the main window that has no title bar of its own, on the
//! multi-viewport harness (`support/multi.rs`): its top tab bar has a "Close window" button
//! beside the move handle, which asks `TabViewer::close_policy` as the title bar's close button
//! does, whether or not its tabs can be closed.

#[path = "support/multi.rs"]
mod multi;

use egui::{Id, Pos2, Rect, Ui, UiBuilder, ViewportId, WidgetText, pos2};
use egui_stack::layout::{Content, Dir, FloatHost, Layout, Zone};
use egui_stack::{
    ClosePolicy, DockArea, DockRules, DockSpace, Docked, FloatClose, FloatEvent, TabViewer,
    ViewportHost,
};
use multi::Multi;

const MAIN: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(300.0, 300.0));
const SIDE: Rect = Rect::from_min_max(pos2(300.0, 0.0), pos2(600.0, 300.0));
const OUTSIDE: Pos2 = pos2(320.0, 420.0);
const CLOSE: &str = "Close window";

/// Tabs that cannot be closed, and the policy a float's close gets.
struct Viewer {
    tab_rects: Vec<(String, Rect)>,
    policy: ClosePolicy,
    asked: usize,
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
        false
    }

    fn close_policy(&mut self, _close: &FloatClose) -> ClosePolicy {
        self.asked += 1;
        self.policy.clone()
    }

    fn on_tab_button(&mut self, tab: &mut String, response: &egui::Response) {
        self.tab_rects.push((tab.clone(), response.rect));
    }
}

struct Host;

impl ViewportHost for Host {}

struct App {
    docked: Docked<String>,
    viewer: Viewer,
    title_bar: bool,
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
            policy,
            asked: 0,
        },
        title_bar: false,
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
        app.docked
            .floats()
            .host(&mut host)
            .title_bar(app.title_bar)
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
