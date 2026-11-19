//! The floats' work area (`FloatsArea::work_area`, `WorkArea`), on the multi-viewport harness
//! (`support/multi.rs`): a tab released outside every region is refused; a float over the main
//! window stays in one region, moved from one to another without straddling the gap, never
//! larger than it, and brought back when its region shrinks or goes, or when a layout is
//! restored with it outside; native floats are left alone. Without a work area nothing changes.
//! With `Outside::Native`, a tab or a float released past the work area's edge becomes a native
//! float where it was released (`Cancel` where viewports are embedded), and a native float's
//! docks still take drops.

#[path = "support/multi.rs"]
mod multi;

use egui::{Event, Id, Pos2, Rect, Ui, UiBuilder, ViewportId, WidgetText, pos2, vec2};
use egui_stack::layout::{Content, Dir, FloatHost, Laid, Layout, Screen, Solve, Zone};
use egui_stack::{
    DockArea, DockRules, DockSpace, Docked, FloatEvent, NodeIndex, TabIndex, TabPath, TabViewer,
    ViewportHost,
};
use egui_stack::{Outside, WorkArea};
use multi::{Multi, SIZE};

const LABEL: &str = "Move window";

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

impl ViewportHost for Host {
    fn screens(&self) -> Vec<Screen> {
        vec![Screen::new(
            "one",
            Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0)),
            1.0,
        )]
    }
}

/// Which work area the application gives.
#[derive(Clone)]
enum Area {
    None,
    Zones,
    /// The zones, a release outside them tearing out to a native window.
    Native,
    Rect(Rect),
}

struct App {
    docked: Docked<String>,
    viewer: Viewer,
    area: Area,
    laid: Laid,
}

/// Three empty zones on top (`left`, `gap`, `right`), each taking a released tab as a float,
/// and the dock `main` below, holding `A`, `B` and `C`.
fn app(host: FloatHost, area: Area) -> App {
    let mut space = DockSpace::new().with_float_host(host);
    let tabs = |names: &[&str]| names.iter().map(|n| (*n).to_owned()).collect();
    space.add_dock("main", DockRules::default(), tabs(&["A", "B", "C"]));
    let layout = Layout::new(Zone::split(
        "root",
        Dir::Vertical,
        vec![
            Zone::split(
                "top",
                Dir::Horizontal,
                vec![
                    Zone::leaf("left", Content::Empty),
                    Zone::leaf("gap", Content::Empty),
                    Zone::leaf("right", Content::Empty),
                ],
            ),
            Zone::leaf("bottom", Content::Dock("main".into())),
        ],
    ));
    App {
        docked: Docked::new(layout, space),
        viewer: Viewer::default(),
        area,
        laid: Laid::default(),
    }
}

fn frame(app: &mut App) -> impl FnMut(&mut Ui) + '_ {
    move |ui| {
        app.viewer.tab_rects.clear();
        let laid = app
            .docked
            .layout
            .solve(Rect::from_min_size(Pos2::ZERO, SIZE), &Solve::default());
        if let Some(rect) = laid.rect("bottom") {
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                DockArea::dock(&mut app.docked.space, "main").show_inside(ui, &mut app.viewer);
            });
        }
        app.docked.space.bind(&laid);
        let area = match &app.area {
            Area::None => WorkArea::window(),
            Area::Zones => WorkArea::new()
                .zone("left")
                .zone("right")
                .outside(Outside::Cancel),
            Area::Native => WorkArea::new()
                .zone("left")
                .zone("right")
                .outside(Outside::Native),
            Area::Rect(rect) => WorkArea::new().rect(*rect),
        };
        app.laid = laid;
        app.docked
            .floats()
            .host(&mut Host)
            .work_area(area)
            .show(ui.ctx(), &mut app.viewer);
    }
}

fn zone(app: &App, name: &str) -> Rect {
    app.laid.rect(name).expect("the zone is laid")
}

fn tabs(app: &App) -> Vec<String> {
    tabs_of(app, "main")
}

fn tabs_of(app: &App, dock: &str) -> Vec<String> {
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

fn float_rect(app: &App) -> Rect {
    app.docked
        .layout
        .float("float-1")
        .expect("B floats")
        .place
        .rect
}

/// Whether `inner` lies in `outer`, give or take egui's rounding.
fn inside(outer: Rect, inner: Rect) -> bool {
    outer.expand(1.0).contains_rect(inner)
}

fn moved_events(app: &mut App) -> usize {
    app.docked
        .space
        .take_float_events()
        .into_iter()
        .filter(|event| matches!(event, FloatEvent::Moved { key, .. } if key == "float-1"))
        .count()
}

/// Starts the application and drags `B` out of the dock to `to`.
fn drag_b_to(multi: &mut Multi, app: &mut App, to: Pos2) {
    multi.passes(3, frame(app));
    let b = app
        .viewer
        .tab_rects
        .iter()
        .find(|(name, _)| name == "B")
        .map(|(_, rect)| rect.center())
        .expect("B is shown");
    multi.drag(b, to, frame(app));
    multi.passes(3, frame(app));
}

/// `B` dragged into the left region: its float, `float-1`.
fn float_in_left(multi: &mut Multi, app: &mut App) {
    multi.passes(2, frame(app));
    let at = zone(app, "left").center();
    drag_b_to(multi, app, at);
    assert!(app.docked.layout.float("float-1").is_some(), "B floats");
    app.docked.space.take_float_events();
}

fn handle(multi: &Multi) -> Pos2 {
    multi
        .find(ViewportId::ROOT, LABEL)
        .expect("the float has a move handle")
        .center()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn without_a_work_area_a_tab_released_anywhere_floats() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Area::None);
    multi.passes(2, frame(&mut app));
    let at = zone(&app, "gap").center();
    drag_b_to(&mut multi, &mut app, at);
    assert!(app.docked.layout.float("float-1").is_some(), "B floats");
    assert_eq!(tabs(&app), ["A", "C"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_released_outside_every_region_stays_where_it_was() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Area::Zones);
    multi.passes(2, frame(&mut app));
    let at = zone(&app, "gap").center();
    drag_b_to(&mut multi, &mut app, at);
    assert!(app.docked.layout.floats.is_empty(), "no float");
    assert_eq!(tabs(&app), ["A", "B", "C"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_released_in_a_region_floats_inside_it() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Area::Zones);
    float_in_left(&mut multi, &mut app);
    assert_eq!(tabs(&app), ["A", "C"]);
    let left = zone(&app, "left");
    assert!(
        inside(left, float_rect(&app)),
        "{left:?} ⊇ {:?}",
        float_rect(&app)
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_moved_past_its_region_stays_inside_it() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Area::Zones);
    float_in_left(&mut multi, &mut app);
    let at = handle(&multi);
    // Over the dock below: no region there.
    let to = pos2(at.x, zone(&app, "bottom").center().y);
    multi.drag(at, to, frame(&mut app));
    let left = zone(&app, "left");
    assert!(
        inside(left, float_rect(&app)),
        "{left:?} ⊇ {:?}",
        float_rect(&app)
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_moved_across_the_gap_switches_region_without_straddling() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Area::Zones);
    float_in_left(&mut multi, &mut app);
    let (left, right) = (zone(&app, "left"), zone(&app, "right"));
    let from = handle(&multi);
    let to = right.center();
    multi.press(ViewportId::ROOT, from, frame(&mut app));
    let mut regions = Vec::new();
    for step in 1..=24 {
        let at = from + (to - from) * (step as f32 / 24.0);
        multi.queue(ViewportId::ROOT, Event::PointerMoved(at));
        multi.pass(frame(&mut app));
        let rect = float_rect(&app);
        let region = if inside(left, rect) {
            "left"
        } else if inside(right, rect) {
            "right"
        } else {
            panic!("step {step}: {rect:?} straddles {left:?} and {right:?}");
        };
        if regions.last() != Some(&region) {
            regions.push(region);
        }
    }
    multi.release(ViewportId::ROOT, to, frame(&mut app));
    multi.passes(3, frame(&mut app));
    // One switch, at the boundary: no flicker back and forth.
    assert_eq!(regions, ["left", "right"]);
    assert!(inside(right, float_rect(&app)));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_is_not_resized_past_its_region() {
    let mut multi = Multi::new();
    let region = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 330.0));
    let mut app = app(FloatHost::InWindow, Area::Rect(region));
    multi.passes(2, frame(&mut app));
    drag_b_to(&mut multi, &mut app, pos2(20.0, 20.0));
    multi.passes(3, frame(&mut app));
    let before = float_rect(&app);
    assert!(inside(region, before), "{region:?} ⊇ {before:?}");
    // The resize grip, at the window's bottom right corner, dragged far below the region.
    let corner = before.max - vec2(2.0, 2.0);
    multi.drag(corner, pos2(corner.x, SIZE.y - 2.0), frame(&mut app));
    let after = float_rect(&app);
    assert!(
        after.height() > before.height() + 30.0,
        "resized: {before:?} → {after:?}"
    );
    assert!(inside(region, after), "{region:?} ⊇ {after:?}");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_whose_region_shrinks_is_brought_back_and_reported_once() {
    let mut multi = Multi::new();
    let wide = Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0));
    let mut app = app(FloatHost::InWindow, Area::Rect(wide));
    multi.passes(2, frame(&mut app));
    drag_b_to(&mut multi, &mut app, pos2(250.0, 150.0));
    multi.passes(3, frame(&mut app));
    app.docked.space.take_float_events();
    let narrow = Rect::from_min_size(Pos2::ZERO, vec2(150.0, 120.0));
    app.area = Area::Rect(narrow);
    multi.passes(6, frame(&mut app));
    let rect = float_rect(&app);
    assert!(inside(narrow, rect), "{narrow:?} ⊇ {rect:?}");
    assert_eq!(moved_events(&mut app), 1);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_whose_zone_goes_moves_into_the_nearest_region() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Area::Zones);
    float_in_left(&mut multi, &mut app);
    app.docked.layout.zone_mut("left").unwrap().state.hidden = true;
    multi.passes(6, frame(&mut app));
    let right = zone(&app, "right");
    let rect = float_rect(&app);
    assert!(inside(right, rect), "{right:?} ⊇ {rect:?}");
    assert_eq!(moved_events(&mut app), 1);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_layout_restored_with_a_float_outside_brings_it_back() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Area::Zones);
    float_in_left(&mut multi, &mut app);
    let mut restored = app.docked.layout.clone();
    restored.float_mut("float-1").unwrap().place.rect =
        Rect::from_min_size(pos2(250.0, 300.0), vec2(500.0, 400.0));
    app.docked.layout = restored;
    multi.passes(6, frame(&mut app));
    let (left, right) = (zone(&app, "left"), zone(&app, "right"));
    let rect = float_rect(&app);
    assert!(
        inside(left, rect) || inside(right, rect),
        "{rect:?} in {left:?} or {right:?}"
    );
    assert_eq!(moved_events(&mut app), 1);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_native_float_ignores_the_work_area() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, Area::Zones);
    float_in_left(&mut multi, &mut app);
    let far = Rect::from_min_size(pos2(900.0, 600.0), vec2(300.0, 200.0));
    app.docked.layout.float_mut("float-1").unwrap().place.rect = far;
    multi.passes(6, frame(&mut app));
    assert_eq!(float_rect(&app).min, far.min);
}

fn host_of(app: &App) -> FloatHost {
    app.docked.layout.float("float-1").expect("B floats").host
}

fn viewport(key: &str) -> ViewportId {
    Host.viewport_id(key)
}

/// Past the main window's right edge.
const PAST_THE_EDGE: Pos2 = pos2(SIZE.x + 60.0, 100.0);

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_released_past_the_work_area_becomes_a_native_float_there() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Area::Native);
    multi.passes(2, frame(&mut app));
    drag_b_to(&mut multi, &mut app, PAST_THE_EDGE);
    assert_eq!(host_of(&app), FloatHost::Native);
    let place = &app.docked.layout.float("float-1").unwrap().place;
    assert_eq!(place.rect.min, PAST_THE_EDGE, "where it was released");
    assert_eq!(place.screen.as_deref(), Some("one"));
    assert_eq!(tabs(&app), ["A", "C"]);
    assert_eq!(tabs_of(&app, "float-1"), ["B"]);
    assert!(multi.shown().contains(&viewport("float-1")), "its window");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_released_in_a_region_still_floats_inside_it() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Area::Native);
    float_in_left(&mut multi, &mut app);
    assert_eq!(host_of(&app), FloatHost::InWindow);
    assert_eq!(tabs(&app), ["A", "C"]);
    let left = zone(&app, "left");
    assert!(
        inside(left, float_rect(&app)),
        "{left:?} ⊇ {:?}",
        float_rect(&app)
    );
    assert!(!multi.shown().contains(&viewport("float-1")), "no window");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_dragged_past_the_edge_tears_out_on_release() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow, Area::Native);
    float_in_left(&mut multi, &mut app);
    let from = handle(&multi);
    let to = pos2(SIZE.x + 80.0, from.y);
    multi.press(ViewportId::ROOT, from, frame(&mut app));
    multi.move_held(ViewportId::ROOT, from, to, frame(&mut app));
    // Held past the edge: still over the main window, bounded by its region.
    assert_eq!(host_of(&app), FloatHost::InWindow);
    let (left, right) = (zone(&app, "left"), zone(&app, "right"));
    let rect = float_rect(&app);
    assert!(inside(left, rect) || inside(right, rect), "{rect:?}");
    multi.release(ViewportId::ROOT, to, frame(&mut app));
    multi.passes(3, frame(&mut app));
    assert_eq!(host_of(&app), FloatHost::Native);
    let rect = float_rect(&app);
    assert!(
        rect.expand(1.0).contains(to),
        "opened where it was let go, under the pointer: {rect:?} ∌ {to:?}"
    );
    assert!(rect.min.x > SIZE.x - rect.width(), "{rect:?}");
    assert_eq!(tabs_of(&app, "float-1"), ["B"]);
    assert!(multi.shown().contains(&viewport("float-1")), "its window");
    let events = app.docked.space.take_float_events();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, FloatEvent::Moved { key, .. } if key == "float-1")),
        "{events:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn with_embedded_viewports_native_falls_back_to_cancel() {
    let mut multi = Multi::new();
    multi.ctx().set_embed_viewports(true);
    let mut app = app(FloatHost::InWindow, Area::Native);
    multi.passes(2, frame(&mut app));
    drag_b_to(&mut multi, &mut app, PAST_THE_EDGE);
    assert!(app.docked.layout.floats.is_empty(), "no float");
    assert_eq!(tabs(&app), ["A", "B", "C"]);

    // A float over the main window let go past the edge stays over it.
    float_in_left(&mut multi, &mut app);
    let from = handle(&multi);
    multi.drag(from, pos2(SIZE.x + 80.0, from.y), frame(&mut app));
    assert_eq!(host_of(&app), FloatHost::InWindow);
    let (left, right) = (zone(&app, "left"), zone(&app, "right"));
    let rect = float_rect(&app);
    assert!(inside(left, rect) || inside(right, rect), "{rect:?}");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_native_floats_docks_take_drops_whatever_the_work_area() {
    for area in [Area::Zones, Area::Native] {
        let mut multi = Multi::new();
        let mut app = app(FloatHost::Native, area);
        multi.passes(3, frame(&mut app));
        // `C` in a native float, its window at (700, 100) on the desktop: past the main
        // window, outside every region.
        let main = app.docked.space.dock_surface("main").unwrap();
        let key = app
            .docked
            .space
            .float_out(
                TabPath::new(main, NodeIndex::root(), TabIndex(2)),
                Rect::from_min_size(pos2(100.0, 100.0), vec2(300.0, 200.0)),
                |tab| Id::unique(tab.as_str()),
            )
            .unwrap();
        multi.pass(frame(&mut app));
        let at = pos2(700.0, 100.0);
        multi.place(viewport(&key), at, 1.0);
        multi.passes(5, frame(&mut app));
        app.docked.space.take_float_events();

        let b = app
            .viewer
            .tab_rects
            .iter()
            .find(|(name, _)| name == "B")
            .map(|(_, rect)| rect.center())
            .expect("B is shown");
        let over = at + vec2(150.0, 100.0);
        multi.drag(b, over, frame(&mut app));
        multi.passes(3, frame(&mut app));
        let mut taken = tabs_of(&app, &key);
        taken.sort();
        assert_eq!(taken, ["B", "C"]);
        assert_eq!(tabs(&app), ["A"]);
        assert!(app.docked.layout.float("float-2").is_none(), "no new float");
    }
}
