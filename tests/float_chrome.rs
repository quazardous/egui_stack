//! What a float looks like around its dock, on the multi-viewport harness (`support/multi.rs`):
//! a float over the main window has no title bar of its own by default, its top tab bar is its
//! top (`FloatsArea::title_bar` brings the title bar back, and a float without its move handle
//! keeps it to be moved by).

#[path = "support/multi.rs"]
mod multi;

use egui::{Id, Pos2, Rect, Ui, UiBuilder, ViewportId, WidgetText, pos2, vec2};
use egui_stack::layout::{Content, Dir, FloatHost, Layout, Zone};
use egui_stack::{
    DockArea, DockRules, DockSpace, Docked, FloatShown, GroupHandles, TabViewer, ViewportHost,
};
use multi::Multi;

const MAIN: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(300.0, 300.0));
const SIDE: Rect = Rect::from_min_max(pos2(300.0, 0.0), pos2(600.0, 300.0));
const OUTSIDE: Pos2 = pos2(320.0, 420.0);

#[derive(Default)]
struct Viewer {
    tab_rects: Vec<(String, Rect)>,
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
}

struct Host;

impl ViewportHost for Host {}

struct App {
    docked: Docked<String>,
    viewer: Viewer,
    /// `FloatsArea::title_bar`, when set.
    title_bar: Option<bool>,
    /// `GroupHandles::MOVE_WINDOW`.
    move_handle: bool,
    shown: Vec<FloatShown>,
}

fn app(host: FloatHost, title_bar: Option<bool>) -> App {
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
        title_bar,
        move_handle: true,
        shown: Vec::new(),
    }
}

fn frame(app: &mut App) -> impl FnMut(&mut Ui) + '_ {
    move |ui| {
        if ui.ctx().viewport_id() == ViewportId::ROOT {
            app.viewer.tab_rects.clear();
            app.shown.clear();
        }
        for (name, rect) in [("main", MAIN), ("side", SIDE)] {
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                DockArea::dock(&mut app.docked.space, name).show_inside(ui, &mut app.viewer);
            });
        }
        let shown = &mut app.shown;
        let mut host = Host;
        let mut floats = app
            .docked
            .floats()
            .host(&mut host)
            .group_handles(GroupHandles::ALL.with(GroupHandles::MOVE_WINDOW, app.move_handle))
            .decorate(|_ui, float| shown.push(float.clone()));
        if let Some(title_bar) = app.title_bar {
            floats = floats.title_bar(title_bar);
        }
        floats.show(ui.ctx(), &mut app.viewer);
    }
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

/// The float's window and its tab bar, over the main window.
fn window_and_bar(app: &App) -> (Rect, Rect) {
    let window = app.docked.layout.float("float-1").unwrap().place.rect;
    let shown = app
        .shown
        .iter()
        .find(|s| s.key == "float-1")
        .expect("the float is decorated");
    let [(_, bar)] = shown.tab_bars[..] else {
        panic!("one bar: {shown:?}");
    };
    (window, bar)
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_over_the_main_window_has_no_title_bar_by_default() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, None);
    detach_b(&mut multi, &mut app);
    let (window, bar) = window_and_bar(&app);
    // The window's frame only (its margin and stroke): no title bar above the tab bar.
    assert!(
        bar.top() - window.top() < 12.0,
        "the tab bar {bar:?} is the top of the window {window:?}"
    );

    // The move handle still moves it.
    let handle = multi
        .find(ViewportId::ROOT, "Move window")
        .unwrap()
        .center();
    multi.drag(handle, handle - vec2(40.0, 30.0), frame(&mut app));
    let moved = app.docked.layout.float("float-1").unwrap().place.rect;
    assert_eq!(
        moved.min,
        window.min - vec2(40.0, 30.0),
        "moved by its handle"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn title_bar_brings_back_the_title_bar_of_a_float_over_the_main_window() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Some(true));
    detach_b(&mut multi, &mut app);
    let (window, bar) = window_and_bar(&app);
    assert!(
        bar.top() - window.top() > 16.0,
        "a title bar above the tab bar {bar:?} in the window {window:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_over_the_main_window_without_its_move_handle_keeps_its_title_bar() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, None);
    app.move_handle = false;
    detach_b(&mut multi, &mut app);
    let (window, bar) = window_and_bar(&app);
    assert!(
        bar.top() - window.top() > 16.0,
        "a title bar to move it by, above the tab bar {bar:?} in the window {window:?}"
    );
}
