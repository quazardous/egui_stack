//! What a float looks like around its dock, on the multi-viewport harness (`support/multi.rs`):
//! a float over the main window has no title bar of its own by default, its top tab bar is its
//! top (`FloatsArea::title_bar` brings the title bar back, and a float without its move handle
//! keeps it to be moved by), and every button of a float's tab bar (collapse, group grip, close
//! all, move window) keeps the line under the bar, at rest and hovered, as the close all button
//! does.

use crate::multi;

use egui::epaint::ClippedShape;
use egui::{Id, Pos2, Rect, Shape, Ui, UiBuilder, ViewportId, WidgetText, pos2, vec2};
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

/// Whether a horizontal line of `shapes` covers `x` (inset by a point at each end) at `y`.
fn line_under(shapes: &[ClippedShape], x: egui::Rangef, y: f32) -> bool {
    fn covers(shape: &Shape, x: egui::Rangef, y: f32) -> bool {
        match shape {
            Shape::LineSegment { points: [a, b], .. } => {
                (a.y - y).abs() < 0.6
                    && (b.y - y).abs() < 0.6
                    && a.x.min(b.x) <= x.min + 1.0
                    && a.x.max(b.x) >= x.max - 1.0
            }
            Shape::Vec(shapes) => shapes.iter().any(|s| covers(s, x, y)),
            _ => false,
        }
    }
    shapes.iter().any(|c| covers(&c.shape, x, y))
}

/// The filled rects of `shapes` over `button`, but its hover background.
fn fills_over(shapes: &[ClippedShape], button: Rect) -> Vec<Rect> {
    fn walk(shape: &Shape, button: Rect, out: &mut Vec<Rect>) {
        match shape {
            Shape::Rect(rect)
                if rect.fill.a() > 0
                    && button.contains_rect(rect.rect.shrink(0.5))
                    && rect.rect.area() > 4.0 =>
            {
                out.push(rect.rect);
            }
            Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, button, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes.iter().for_each(|c| walk(&c.shape, button, &mut out));
    out
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn every_button_of_a_float_s_tab_bar_keeps_the_line_under_the_bar() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, None);
    detach_b(&mut multi, &mut app);
    let viewport = Host.viewport_id("float-1");
    multi.passes(2, frame(&mut app));
    let shown = app
        .shown
        .iter()
        .find(|s| s.key == "float-1")
        .expect("the float is decorated")
        .clone();
    let [(_, bar)] = shown.tab_bars[..] else {
        panic!("one bar: {shown:?}");
    };
    let line_y = bar.bottom() - 0.5;
    let labels = ["Collapse", "Move group", "Close all tabs", "Move window"];
    for label in labels {
        let button = multi
            .find(viewport, label)
            .unwrap_or_else(|| panic!("{label} is shown"));
        assert!(
            line_under(multi.shapes(viewport), button.x_range(), line_y),
            "{label}: the line under the bar runs under it, at rest ({button:?} in {bar:?})"
        );
    }
    for label in labels {
        let button = multi.find(viewport, label).unwrap();
        multi.queue(viewport, egui::Event::PointerMoved(button.center()));
        multi.passes(2, frame(&mut app));
        let fills = fills_over(multi.shapes(viewport), button);
        assert!(
            fills.iter().all(|fill| fill.bottom() <= bar.bottom() - 2.0),
            "{label}: its hover background leaves the line under the bar: {fills:?}, bar {bar:?}"
        );
        assert!(
            line_under(multi.shapes(viewport), button.x_range(), line_y),
            "{label}: the line under the bar runs under it, hovered"
        );
    }
}

/// The widths of the line segments drawn over `button` that are not whole physical pixels at
/// `ppp`.
fn blurred_over(shapes: &[ClippedShape], button: Rect, ppp: f32) -> Vec<f32> {
    fn walk(shape: &Shape, button: Rect, ppp: f32, out: &mut Vec<f32>) {
        match shape {
            Shape::LineSegment {
                points: [a, b],
                stroke,
            } if stroke.width > 0.0
                && button.expand(0.5).contains(*a)
                && button.expand(0.5).contains(*b) =>
            {
                let px = stroke.width * ppp;
                if (px - px.round()).abs() > 1e-3 {
                    out.push(stroke.width);
                }
            }
            Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, button, ppp, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    shapes
        .iter()
        .for_each(|c| walk(&c.shape, button, ppp, &mut out));
    out
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_buttons_of_a_float_s_tab_bar_draw_whole_pixels_at_every_scale() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, None);
    detach_b(&mut multi, &mut app);
    let viewport = Host.viewport_id("float-1");
    multi.passes(2, frame(&mut app));
    let labels = ["Collapse", "Move group", "Close all tabs", "Move window"];
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0, 3.0] {
        multi.place(viewport, pos2(400.0, 100.0), scale);
        multi.passes(2, frame(&mut app));
        for label in labels {
            let button = multi
                .find(viewport, label)
                .unwrap_or_else(|| panic!("{label} is shown at {scale}"));
            let blurred = blurred_over(multi.shapes(viewport), button, scale);
            assert!(
                blurred.is_empty(),
                "{label}: strokes {blurred:?} points wide are not whole pixels at {scale}"
            );
        }
    }
}
