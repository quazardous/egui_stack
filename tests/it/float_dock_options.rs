//! What the application sets of the floats' docks through `FloatsArea`, on a harness whose
//! viewports are real (`support/multi.rs`): its painter paints the floats' tab bars and window
//! outlines, `configure` sets each float's `DockArea` options (no splits, no buttons),
//! `decorate` tells where each float's tab bars are, at scales 1 and 2, and
//! `DockSpace::carry_floats_from` keeps the floats of a space rebuilt during a drag.

use crate::multi;

use std::sync::Mutex;

use egui::{Id, Painter, Pos2, Rect, Ui, UiBuilder, ViewportId, WidgetText, pos2, vec2};
use egui_stack::layout::{Content, Dir, FloatHost, Layout, Zone};
use egui_stack::{
    AllowedSplits, DockArea, DockPainter, DockRules, DockSpace, Docked, FloatShown, NodeIndex,
    Style, StylePainter, TabBarPaint, TabIndex, TabPath, TabViewer, ViewportHost,
    WindowOutlinePaint,
};
use multi::Multi;

const MAIN: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(300.0, 300.0));
const SIDE: Rect = Rect::from_min_max(pos2(300.0, 0.0), pos2(600.0, 300.0));

/// A float's size, in points.
const FLOAT: egui::Vec2 = vec2(300.0, 200.0);

#[derive(Default)]
struct Viewer {
    tab_rects: Vec<(String, ViewportId, Rect)>,
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
        self.tab_rects
            .push((tab.clone(), response.ctx.viewport_id(), response.rect));
    }
}

struct Host;

impl ViewportHost for Host {}

/// A painter recording, by viewport, the tab bars and window outlines it paints, then painting
/// them as the default does.
#[derive(Default)]
struct Recorder {
    tab_bars: Mutex<Vec<(ViewportId, Rect)>>,
    outlines: Mutex<Vec<(ViewportId, Rect)>>,
}

impl DockPainter for Recorder {
    fn tab_bar(&self, painter: &Painter, bar: &TabBarPaint<'_>) {
        self.tab_bars
            .lock()
            .unwrap()
            .push((painter.ctx().viewport_id(), bar.rect));
        StylePainter.tab_bar(painter, bar);
    }

    fn window_outline(&self, painter: &Painter, outline: &WindowOutlinePaint<'_>) {
        self.outlines
            .lock()
            .unwrap()
            .push((painter.ctx().viewport_id(), outline.rect));
        StylePainter.window_outline(painter, outline);
    }
}

/// What the test sets of the floats.
#[derive(Default)]
struct Options {
    painter: Option<&'static Recorder>,
    /// No splits and no buttons in the floats' docks.
    bare: bool,
}

struct App {
    docked: Docked<String>,
    viewer: Viewer,
    options: Options,
    /// The floats each pass decorated, and the keys `configure` was called with.
    shown: Vec<FloatShown>,
    configured: Vec<String>,
}

fn strings(tabs: &[&str]) -> Vec<String> {
    tabs.iter().map(|tab| (*tab).to_owned()).collect()
}

fn layout() -> Layout {
    Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::leaf("main", Content::Dock("main".into())),
            Zone::leaf("side", Content::Dock("side".into())),
        ],
    ))
}

/// `A`, `B`, `C` in the main dock, `S` in the side dock; floats hosted by `host`.
fn app(host: FloatHost, options: Options) -> App {
    let mut space = DockSpace::new().with_float_host(host);
    space.add_dock("main", DockRules::default(), strings(&["A", "B", "C"]));
    space.add_dock("side", DockRules::default(), strings(&["S"]));
    App {
        docked: Docked::new(layout(), space),
        viewer: Viewer::default(),
        options,
        shown: Vec::new(),
        configured: Vec::new(),
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
        let bare = app.options.bare;
        let shown = &mut app.shown;
        let configured = &mut app.configured;
        let mut host = Host;
        let mut floats = app
            .docked
            .floats()
            .host(&mut host)
            .configure(move |float, area| {
                configured.push(float.key.clone());
                if bare {
                    area.allowed_splits(AllowedSplits::None)
                        .show_close_buttons(false)
                        .show_add_buttons(false)
                        .show_leaf_close_all_buttons(false)
                        .show_leaf_collapse_buttons(false)
                } else {
                    area
                }
            })
            .decorate(|_ui, float| shown.push(float.clone()));
        if let Some(painter) = app.options.painter {
            floats = floats.painter(painter);
        }
        floats.show(ui.ctx(), &mut app.viewer);
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

fn leaves(app: &App, dock: &str) -> usize {
    app.docked
        .space
        .dock(dock)
        .map_or(0, |tree| tree.iter().filter(|n| n.is_leaf()).count())
}

fn tab_rect(app: &App, tab: &str) -> (ViewportId, Rect) {
    app.viewer
        .tab_rects
        .iter()
        .rev()
        .find(|(name, _, _)| name == tab)
        .map(|(_, viewport, rect)| (*viewport, *rect))
        .unwrap_or_else(|| panic!("{tab} is shown"))
}

fn viewport(key: &str) -> ViewportId {
    Host.viewport_id(key)
}

/// Floats the tab `tab` of the main dock out, then, for a native float, puts its window's
/// top-left at `physical` on a screen of scale `scale`. Returns its key.
fn float(multi: &mut Multi, app: &mut App, tab: &str, physical: Pos2, scale: f32) -> String {
    let main = app.docked.space.dock_surface("main").unwrap();
    let index = tabs(app, "main").iter().position(|t| t == tab).unwrap();
    let key = app
        .docked
        .space
        .float_out(
            TabPath::new(main, NodeIndex::root(), TabIndex(index)),
            Rect::from_min_size(pos2(100.0, 100.0), FLOAT),
            |tab| Id::unique(tab.as_str()),
        )
        .unwrap();
    multi.pass(frame(app));
    multi.place(viewport(&key), physical, scale);
    multi.passes(3, frame(app));
    app.docked.space.take_float_events();
    key
}

fn recorder() -> &'static Recorder {
    Box::leak(Box::default())
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_application_painter_paints_the_floats_tab_bars() {
    let painter = recorder();
    let mut multi = Multi::new();
    let mut app = app(
        FloatHost::Native,
        Options {
            painter: Some(painter),
            bare: false,
        },
    );
    multi.passes(3, frame(&mut app));
    float(&mut multi, &mut app, "C", pos2(700.0, 100.0), 1.0);

    let bars = painter.tab_bars.lock().unwrap().clone();
    assert!(
        bars.iter().any(|(v, _)| *v == viewport("float-1")),
        "the float's bar painted by the app's painter: {bars:?}"
    );
    assert!(
        bars.iter().all(|(v, _)| *v != ViewportId::ROOT),
        "the main window's docks keep their own painter: {bars:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_application_painter_outlines_a_float_where_positions_are_hidden() {
    let painter = recorder();
    let mut multi = Multi::new();
    multi.hide_positions();
    let mut app = app(
        FloatHost::Native,
        Options {
            painter: Some(painter),
            bare: false,
        },
    );
    multi.passes(3, frame(&mut app));
    float(&mut multi, &mut app, "C", pos2(700.0, 100.0), 1.0);
    painter.outlines.lock().unwrap().clear();

    let b = tab_rect(&app, "B").1.center();
    multi.press(ViewportId::ROOT, b, frame(&mut app));
    multi.move_held(ViewportId::ROOT, b, pos2(900.0, 200.0), frame(&mut app));

    let outlines = painter.outlines.lock().unwrap().clone();
    assert!(
        outlines.iter().any(|(v, _)| *v == viewport("float-1")),
        "the float's dock outlined by the app's painter: {outlines:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn configure_hides_the_floats_buttons() {
    for bare in [false, true] {
        let mut multi = Multi::new();
        let mut app = app(
            FloatHost::Native,
            Options {
                painter: None,
                bare,
            },
        );
        multi.passes(3, frame(&mut app));
        float(&mut multi, &mut app, "C", pos2(700.0, 100.0), 1.0);

        assert!(app.configured.iter().all(|key| key == "float-1"));
        assert!(!app.configured.is_empty(), "configure called");
        for label in ["Close tab", "Close all tabs", "Collapse"] {
            assert_eq!(
                multi.find(viewport("float-1"), label).is_some(),
                !bare,
                "{label}, bare: {bare}"
            );
            assert!(
                multi.find(ViewportId::ROOT, label).is_some(),
                "{label}: the main window's docks keep theirs"
            );
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn configure_disables_splits_in_a_float() {
    for bare in [false, true] {
        let mut multi = Multi::new();
        let mut app = app(
            FloatHost::Native,
            Options {
                painter: None,
                bare,
            },
        );
        multi.passes(3, frame(&mut app));
        let at = pos2(700.0, 100.0);
        float(&mut multi, &mut app, "C", at, 1.0);

        // `B` dropped near the float's left edge, below its tab bar: a split, if allowed.
        let b = tab_rect(&app, "B").1.center();
        let over = at + vec2(15.0, FLOAT.y / 2.0);
        multi.press(ViewportId::ROOT, b, frame(&mut app));
        multi.move_held(ViewportId::ROOT, b, over, frame(&mut app));
        multi.release(ViewportId::ROOT, over, frame(&mut app));
        multi.passes(3, frame(&mut app));

        let mut landed = tabs(&app, "float-1");
        landed.sort();
        assert_eq!(landed, ["B", "C"], "bare: {bare}");
        assert_eq!(
            leaves(&app, "float-1"),
            if bare { 1 } else { 2 },
            "bare: {bare}"
        );
    }
}

/// The float's tab bar, as `decorate` gets it, is where its tab is, the whole width of its
/// dock and the style's height, at the top of its dock; at `scale`.
fn tab_bar_reported_at(scale: f32) {
    let painter = recorder();
    let mut multi = Multi::new();
    let mut app = app(
        FloatHost::Native,
        Options {
            painter: Some(painter),
            bare: true,
        },
    );
    multi.passes(3, frame(&mut app));
    let at = pos2(2100.0, 200.0);
    float(&mut multi, &mut app, "C", at, scale);
    painter.tab_bars.lock().unwrap().clear();
    multi.pass(frame(&mut app));

    let shown: Vec<_> = app.shown.iter().filter(|s| s.key == "float-1").collect();
    let [shown] = shown[..] else {
        panic!("decorated once: {:?}", app.shown);
    };
    assert_eq!(shown.dock, "float-1");
    assert_eq!(shown.viewport, viewport("float-1"));
    assert!(shown.native);
    let [(path, bar)] = shown.tab_bars[..] else {
        panic!("one bar: {shown:?}");
    };
    assert_eq!(Some(path.surface), app.docked.space.dock_surface("float-1"));
    let height = Style::from_egui(&egui::Style::default()).tab_bar.height;
    assert_eq!(bar.height(), height, "scale {scale}");
    assert_eq!(bar.min, shown.rect.min, "scale {scale}");
    assert_eq!(bar.width(), shown.rect.width(), "scale {scale}");
    let (tab_viewport, tab) = tab_rect(&app, "C");
    assert_eq!(tab_viewport, viewport("float-1"));
    assert!(bar.contains_rect(tab), "{tab:?} in {bar:?}, scale {scale}");
    let painted = painter.tab_bars.lock().unwrap().clone();
    assert_eq!(painted, [(viewport("float-1"), bar)], "scale {scale}");
    assert_eq!(
        multi
            .ctx()
            .input(|i| i.raw.viewports[&viewport("float-1")].native_pixels_per_point),
        Some(scale)
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn decorate_reports_the_tab_bar_at_scale_1() {
    tab_bar_reported_at(1.0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn decorate_reports_the_tab_bar_at_scale_2() {
    tab_bar_reported_at(2.0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn decorate_reports_the_tab_bar_of_a_float_over_the_main_window() {
    let mut multi = Multi::new();
    let mut app = app(
        FloatHost::InWindow,
        Options {
            painter: None,
            bare: true,
        },
    );
    multi.passes(3, frame(&mut app));
    float(&mut multi, &mut app, "C", pos2(0.0, 0.0), 1.0);
    multi.pass(frame(&mut app));

    let [shown] = &app.shown[..] else {
        panic!("decorated once: {:?}", app.shown);
    };
    assert!(!shown.native);
    assert_eq!(shown.viewport, ViewportId::ROOT);
    let [(_, bar)] = shown.tab_bars[..] else {
        panic!("only the float's bar, not the main window's docks': {shown:?}");
    };
    let (_, tab) = tab_rect(&app, "C");
    assert!(bar.contains_rect(tab), "{tab:?} in {bar:?}");
    assert!(shown.rect.contains_rect(bar));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_space_rebuilt_during_a_drag_keeps_its_floats() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, Options::default());
    multi.passes(3, frame(&mut app));
    let at = pos2(700.0, 100.0);
    float(&mut multi, &mut app, "C", at, 1.0);

    let b = tab_rect(&app, "B").1.center();
    let over = at + FLOAT / 2.0;
    multi.press(ViewportId::ROOT, b, frame(&mut app));
    multi.move_held(ViewportId::ROOT, b, over, frame(&mut app));
    assert!(app.docked.space.dragging().is_some());

    // The application rebuilds its space from its own layout, in the same order.
    let mut rebuilt = DockSpace::new().with_id(app.docked.space.id());
    rebuilt.add_dock("main", DockRules::default(), strings(&["A", "B"]));
    rebuilt.add_dock("side", DockRules::default(), strings(&["S"]));
    rebuilt.add_dock("float-1", DockRules::default(), strings(&["C"]));
    rebuilt.carry_floats_from(&mut app.docked.space);
    app.docked.space = rebuilt;
    assert!(app.docked.space.dragging().is_some(), "the drag carried");
    assert_eq!(app.docked.space.float_host(), FloatHost::Native);
    assert!(
        app.docked.space.origin(Id::unique("C")).is_some(),
        "the origins carried"
    );

    multi.release(ViewportId::ROOT, over, frame(&mut app));
    multi.passes(3, frame(&mut app));
    let mut landed = tabs(&app, "float-1");
    landed.sort();
    assert_eq!(landed, ["B", "C"]);
    assert_eq!(tabs(&app, "main"), ["A"]);
    let builders: Vec<_> = multi.take_commands(viewport("float-1"));
    assert!(
        !builders
            .iter()
            .any(|c| matches!(c, egui::ViewportCommand::OuterPosition(_))),
        "the window was not moved back where it opened: {builders:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_drag_whose_tab_is_gone_from_the_rebuilt_space_is_dropped() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native, Options::default());
    multi.passes(3, frame(&mut app));
    float(&mut multi, &mut app, "C", pos2(700.0, 100.0), 1.0);

    let b = tab_rect(&app, "B").1.center();
    multi.press(ViewportId::ROOT, b, frame(&mut app));
    multi.move_held(ViewportId::ROOT, b, pos2(900.0, 200.0), frame(&mut app));
    assert!(app.docked.space.dragging().is_some());

    let mut rebuilt = DockSpace::new().with_id(app.docked.space.id());
    rebuilt.add_dock("main", DockRules::default(), strings(&["A"]));
    rebuilt.add_dock("side", DockRules::default(), strings(&["S"]));
    rebuilt.add_dock("float-1", DockRules::default(), strings(&["C"]));
    rebuilt.carry_floats_from(&mut app.docked.space);
    assert!(rebuilt.dragging().is_none());
    assert!(
        app.docked.space.dragging().is_none(),
        "nothing left in the old one"
    );
}
