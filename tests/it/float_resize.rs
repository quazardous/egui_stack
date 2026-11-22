//! Resizing a float over the main window by its edges and corners, on the multi-viewport
//! harness (`support/multi.rs`): each edge and corner of its window resizes it, the dragged
//! edges follow the pointer and the opposite ones stay where they were. The left and top
//! edges, which move the window's origin, resize it as the right and bottom ones do.

use crate::multi;

use egui::{Id, Pos2, Rect, Ui, UiBuilder, Vec2, WidgetText, pos2, vec2};
use egui_stack::layout::{Content, FloatHost, Layout, Zone};
use egui_stack::{DockArea, DockRules, DockSpace, Docked, TabViewer, ViewportHost};
use multi::{Multi, SIZE};

/// The main window's dock, at its top middle: the floats have room around it.
const DOCK: Rect = Rect::from_min_max(pos2(200.0, 0.0), pos2(440.0, 120.0));
/// Where `B` is released, out of the dock: its float opens there.
const OUTSIDE: Pos2 = pos2(300.0, 260.0);

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
    title_bar: bool,
}

fn app() -> App {
    let mut space = DockSpace::new().with_float_host(FloatHost::InWindow);
    let tabs = ["A", "B", "C"].map(str::to_owned).to_vec();
    space.add_dock("main", DockRules::default(), tabs);
    let layout = Layout::new(Zone::leaf("main", Content::Dock("main".into())));
    App {
        docked: Docked::new(layout, space),
        viewer: Viewer::default(),
        title_bar: false,
    }
}

fn frame(app: &mut App) -> impl FnMut(&mut Ui) + '_ {
    move |ui| {
        app.viewer.tab_rects.clear();
        ui.scope_builder(UiBuilder::new().max_rect(DOCK), |ui| {
            DockArea::dock(&mut app.docked.space, "main").show_inside(ui, &mut app.viewer);
        });
        let mut host = Host;
        app.docked
            .floats()
            .host(&mut host)
            .title_bar(app.title_bar)
            .show(ui.ctx(), &mut app.viewer);
    }
}

/// Starts the application and drags `B` out of the dock: its float, `float-1`, which it
/// returns the window's rect of.
fn detach_b(multi: &mut Multi, app: &mut App) -> Rect {
    multi.passes(3, frame(app));
    let b = app
        .viewer
        .tab_rects
        .iter()
        .find(|(tab, _)| tab == "B")
        .expect("B is shown")
        .1
        .center();
    multi.drag(b, OUTSIDE, frame(app));
    multi.passes(3, frame(app));
    float_rect(app)
}

fn float_rect(app: &App) -> Rect {
    app.docked
        .layout
        .float("float-1")
        .expect("B floats")
        .place
        .rect
}

/// A side of the window: -1 its left or top edge, 1 its right or bottom one, 0 neither.
type Grab = (f32, f32);

/// The point of `rect` to grab for `grab`: just inside the edge, at its middle along it.
fn grab_point(rect: Rect, (x, y): Grab) -> Pos2 {
    let along = |sign: f32, min: f32, max: f32| match sign {
        s if s < 0.0 => min + 1.0,
        s if s > 0.0 => max - 1.0,
        _ => (min + max) / 2.0,
    };
    pos2(
        along(x, rect.left(), rect.right()),
        along(y, rect.top(), rect.bottom()),
    )
}

/// Drags the window's `grab` by `by` (outwards for a positive `by` on that side) and checks
/// the dragged edges moved by it, the others stayed.
fn check_resize(grab: Grab, title_bar: bool) {
    let mut multi = Multi::new();
    let mut app = app();
    app.title_bar = title_bar;
    let before = detach_b(&mut multi, &mut app);
    assert!(
        before.min.x > 60.0 && before.min.y > DOCK.max.y + 60.0 && before.max.x < SIZE.x - 60.0,
        "room around the float: {before:?}"
    );
    let delta = vec2(grab.0, grab.1) * 40.0;
    let from = grab_point(before, grab);
    multi.drag(from, from + delta, frame(&mut app));
    let after = float_rect(&app);
    let what = format!("grab {grab:?}, title bar {title_bar}: {before:?} -> {after:?}");
    let moved = |was: f32, is: f32, by: f32| (is - was - by).abs() <= 2.0;
    let (dx, dy) = (delta.x, delta.y);
    assert!(
        moved(before.left(), after.left(), dx.min(0.0)),
        "the left edge: {what}"
    );
    assert!(
        moved(before.right(), after.right(), dx.max(0.0)),
        "the right edge: {what}"
    );
    assert!(
        moved(before.top(), after.top(), dy.min(0.0)),
        "the top edge: {what}"
    );
    assert!(
        moved(before.bottom(), after.bottom(), dy.max(0.0)),
        "the bottom edge: {what}"
    );
    assert_eq!(
        after.size() - before.size(),
        Vec2::new(dx.abs(), dy.abs()),
        "the size grows by the drag: {what}"
    );
}

/// Every edge and corner, the float with or without its title bar.
#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_resizes_by_each_edge_and_corner() {
    let mut failures = Vec::new();
    for title_bar in [false, true] {
        for grab in [
            (-1.0, 0.0),
            (1.0, 0.0),
            (0.0, -1.0),
            (0.0, 1.0),
            (-1.0, -1.0),
            (1.0, -1.0),
            (-1.0, 1.0),
            (1.0, 1.0),
        ] {
            let result = std::panic::catch_unwind(|| check_resize(grab, title_bar));
            if let Err(err) = result {
                let msg = err.downcast_ref::<String>().cloned().unwrap_or_default();
                failures.push(msg);
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
