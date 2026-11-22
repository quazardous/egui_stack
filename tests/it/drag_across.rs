//! One drag across viewports (`DockSpace::dragging`), on a harness whose viewports are real
//! (`support/multi.rs`): a tab or a group dragged from the main window to a native float, from
//! a float to the main window or to another float, released over nothing (a new float, or back
//! where it was when refused), the veto, Escape, the target window closed during the drag,
//! window positions hidden (Wayland), and screens of scale 1 and 2.
//!
//! As the system does, the window a drag began in gets every pointer event of the drag, its
//! pointer following the drag past its edges; no other window sees the drag or its release.

use crate::multi;

use egui::{
    Event, Id, Key, Modifiers, Pos2, Rect, Ui, UiBuilder, ViewportCommand, ViewportId, WidgetText,
    pos2, vec2,
};
use egui_stack::layout::{Content, Dir, FloatHost, Layout, Screen, Zone};
use egui_stack::{
    DockArea, DockDrop, DockRules, DockSpace, Docked, DragPayload, FloatEvent, NodeIndex,
    TabDestination, TabIndex, TabOrigin, TabPath, TabViewer, ViewportHost, tab_viewer::DropVerdict,
};
use multi::Multi;

/// Where the main dock is drawn in the main window (640 × 480 at the desktop's corner); the
/// side dock is on its right.
const MAIN: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(300.0, 300.0));
const SIDE: Rect = Rect::from_min_max(pos2(300.0, 0.0), pos2(600.0, 300.0));

/// A float's size, in points.
const FLOAT: egui::Vec2 = vec2(300.0, 200.0);

#[derive(Default)]
struct Viewer {
    tab_rects: Vec<(String, Rect)>,
    /// Refuses drops into floats' docks.
    refuse_floats: bool,
    /// Refuses new windows.
    refuse_windows: bool,
    /// Paints the ghosts' bodies (`TabViewer::ghost_body`) in [`PREVIEW`].
    preview: bool,
}

/// The colour of a previewing viewer's ghost body.
const PREVIEW: egui::Color32 = egui::Color32::from_rgb(0x12, 0x34, 0x56);

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

    fn ghost_body(&mut self, ui: &mut Ui, _tab: &String, rect: Rect) -> bool {
        if self.preview {
            ui.painter().rect_filled(rect.shrink(4.0), 0.0, PREVIEW);
        }
        self.preview
    }

    fn dock_drop_allowed(&self, _tab: &String, drop: &DockDrop<'_>) -> DropVerdict {
        let into_float = drop.to.is_some_and(|to| to.starts_with("float-"));
        let new_window = matches!(drop.destination, TabDestination::Window(_));
        if (self.refuse_floats && into_float) || (self.refuse_windows && new_window) {
            DropVerdict::Refuse(Some("not there".into()))
        } else {
            DropVerdict::Allow
        }
    }
}

struct Host;

impl ViewportHost for Host {
    fn screens(&self) -> Vec<Screen> {
        vec![one(), two()]
    }
}

fn one() -> Screen {
    Screen::new(
        "one",
        Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0)),
        1.0,
    )
}

fn two() -> Screen {
    Screen::new(
        "two",
        Rect::from_min_size(pos2(1920.0, 0.0), vec2(2560.0, 1440.0)),
        2.0,
    )
}

struct App {
    docked: Docked<String>,
    viewer: Viewer,
}

fn strings(tabs: &[&str]) -> Vec<String> {
    tabs.iter().map(|tab| (*tab).to_owned()).collect()
}

/// `A`, `B`, `C` in the main dock, `S` in the side dock; floats native.
fn app() -> App {
    let mut space = DockSpace::new().with_float_host(FloatHost::Native);
    space.add_dock("main", DockRules::default(), strings(&["A", "B", "C"]));
    space.add_dock("side", DockRules::default(), strings(&["S"]));
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
        app.docked
            .floats()
            .host(&mut Host)
            .show(ui.ctx(), &mut app.viewer);
    }
}

fn tab_rect(app: &App, tab: &str) -> Rect {
    app.viewer
        .tab_rects
        .iter()
        .rev()
        .find(|(name, _)| name == tab)
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("{tab} is shown"))
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

fn sorted(mut tabs: Vec<String>) -> Vec<String> {
    tabs.sort();
    tabs
}

fn viewport(key: &str) -> ViewportId {
    Host.viewport_id(key)
}

/// The middle of the side dock's leaf, below its tab bar, on the desktop (the main window is
/// at the desktop's corner, at scale 1).
fn side_middle() -> Pos2 {
    pos2(SIDE.center().x, 150.0)
}

/// Floats the tab `tab` of the main dock out into a native float, then puts its window's
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

/// The middle of a float's window on the desktop, in physical pixels, its top-left at
/// `physical` on a screen of scale `scale`.
fn float_middle(physical: Pos2, scale: f32) -> Pos2 {
    physical + FLOAT * scale / 2.0
}

/// A desktop point (physical pixels) in the points of a window whose top-left is at `physical`
/// on a screen of scale `scale`.
fn local(desktop: Pos2, physical: Pos2, scale: f32) -> Pos2 {
    ((desktop - physical) / scale).to_pos2()
}

/// Starts the application with `C` in a float `float-1` at (700, 100) on the screen `one`.
fn with_float(multi: &mut Multi, app: &mut App) -> Pos2 {
    multi.passes(3, frame(app));
    let at = pos2(700.0, 100.0);
    assert_eq!(float(multi, app, "C", at, 1.0), "float-1");
    multi.passes(2, frame(app));
    at
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_dragged_from_the_main_window_lands_in_a_native_float() {
    let mut multi = Multi::new();
    let mut app = app();
    let at = with_float(&mut multi, &mut app);

    let b = tab_rect(&app, "B").center();
    let over = float_middle(at, 1.0);
    multi.press(ViewportId::ROOT, b, frame(&mut app));
    multi.move_held(ViewportId::ROOT, b, over, frame(&mut app));
    let dragging = app.docked.space.dragging().expect("a drag");
    assert_eq!(dragging.source, ViewportId::ROOT);
    assert_eq!(dragging.over, Some(viewport("float-1")), "{dragging:?}");
    assert!(matches!(dragging.payload, DragPayload::Tab(_)));
    multi.release(ViewportId::ROOT, over, frame(&mut app));
    multi.passes(3, frame(&mut app));

    assert_eq!(sorted(tabs(&app, "float-1")), ["B", "C"]);
    assert_eq!(tabs(&app, "main"), ["A"]);
    assert!(app.docked.layout.float("float-2").is_none(), "no new float");
    assert_eq!(
        app.docked.space.origin(Id::unique("B")),
        Some(&TabOrigin::new(
            Id::unique("B"),
            "main",
            Some(Id::unique("A")),
            None
        )),
        "its origin recorded"
    );
    assert!(app.docked.space.dragging().is_none());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_dragged_from_a_native_float_lands_in_the_main_window() {
    let mut multi = Multi::new();
    let mut app = app();
    let at = with_float(&mut multi, &mut app);

    let c = tab_rect(&app, "C").center();
    let to = local(side_middle(), at, 1.0);
    assert!(to.x < 0.0, "past the float's edge");
    multi.drag_in(viewport("float-1"), c, to, frame(&mut app));

    assert_eq!(sorted(tabs(&app, "side")), ["C", "S"]);
    assert!(
        app.docked.layout.float("float-1").is_none(),
        "emptied, closed"
    );
    assert!(!multi.shown().contains(&viewport("float-1")));
    assert_eq!(app.docked.space.origin(Id::unique("C")), None, "back home");
    assert_eq!(tabs(&app, "main"), ["A", "B"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_dragged_from_a_float_lands_in_another_float() {
    let mut multi = Multi::new();
    let mut app = app();
    let first = with_float(&mut multi, &mut app);
    let second = pos2(1100.0, 100.0);
    assert_eq!(float(&mut multi, &mut app, "B", second, 1.0), "float-2");
    multi.passes(2, frame(&mut app));

    let c = tab_rect(&app, "C").center();
    let to = local(float_middle(second, 1.0), first, 1.0);
    multi.drag_in(viewport("float-1"), c, to, frame(&mut app));

    assert_eq!(sorted(tabs(&app, "float-2")), ["B", "C"]);
    assert!(app.docked.layout.float("float-1").is_none());
    assert_eq!(tabs(&app, "main"), ["A"]);
    assert_eq!(
        app.docked
            .space
            .origin(Id::unique("C"))
            .map(|o| o.dock.as_str()),
        Some("main"),
        "from float to float, its origin kept"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_group_dragged_from_the_main_window_lands_whole_in_a_float() {
    let mut multi = Multi::new();
    let mut app = app();
    let at = with_float(&mut multi, &mut app);

    let a = tab_rect(&app, "A").center();
    multi.hold(ViewportId::ROOT, Modifiers::SHIFT);
    multi.press(ViewportId::ROOT, a, frame(&mut app));
    let over = float_middle(at, 1.0);
    multi.move_held(ViewportId::ROOT, a, over, frame(&mut app));
    assert!(matches!(
        app.docked.space.dragging().map(|d| d.payload),
        Some(DragPayload::Group(_))
    ));
    multi.release(ViewportId::ROOT, over, frame(&mut app));
    multi.passes(3, frame(&mut app));

    assert_eq!(sorted(tabs(&app, "float-1")), ["A", "B", "C"]);
    assert!(tabs(&app, "main").is_empty());
    for tab in ["A", "B"] {
        assert_eq!(
            app.docked
                .space
                .origin(Id::unique(tab))
                .map(|o| o.dock.as_str()),
            Some("main"),
            "{tab}'s origin"
        );
    }
}

/// The texts a window painted in the last pass.
fn texts(multi: &Multi, viewport: ViewportId) -> Vec<String> {
    fn visit(shape: &egui::Shape, texts: &mut Vec<String>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| visit(s, texts)),
            egui::Shape::Text(text) => texts.push(text.galley.text().to_owned()),
            _ => {}
        }
    }
    let mut texts = Vec::new();
    for clipped in multi.shapes(viewport) {
        visit(&clipped.shape, &mut texts);
    }
    texts
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_group_dragged_over_a_float_shows_its_ghost_there() {
    let mut multi = Multi::new();
    let mut app = app();
    let at = with_float(&mut multi, &mut app);
    let float = viewport("float-1");
    assert!(
        !texts(&multi, float).iter().any(|t| t == "A" || t == "B"),
        "before the drag, the float shows only its own tab"
    );

    let a = tab_rect(&app, "A").center();
    multi.hold(ViewportId::ROOT, Modifiers::SHIFT);
    multi.press(ViewportId::ROOT, a, frame(&mut app));
    let over = float_middle(at, 1.0);
    multi.move_held(ViewportId::ROOT, a, over, frame(&mut app));
    let shown = texts(&multi, float);
    for tab in ["A", "B"] {
        assert!(
            shown.iter().any(|t| t == tab),
            "the group's ghost, {tab} among its tabs, is painted in the float under the pointer: {shown:?}"
        );
    }
    multi.release(ViewportId::ROOT, over, frame(&mut app));
    multi.hold(ViewportId::ROOT, Modifiers::NONE);
    multi.passes(3, frame(&mut app));
    assert_eq!(sorted(tabs(&app, "float-1")), ["A", "B", "C"]);
}

/// The rects filled with `color` in a window's last pass.
fn filled(multi: &Multi, viewport: ViewportId, color: egui::Color32) -> Vec<Rect> {
    fn visit(shape: &egui::Shape, color: egui::Color32, rects: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| visit(s, color, rects)),
            egui::Shape::Rect(rect) if rect.fill == color => rects.push(rect.rect),
            _ => {}
        }
    }
    let mut rects = Vec::new();
    for clipped in multi.shapes(viewport) {
        visit(&clipped.shape, color, &mut rects);
    }
    rects
}

/// A tab dragged from the main window over a native float on a screen of `scale`: its ghost,
/// with the viewer's body, is painted by the float, at the same size in points, and nothing of
/// it by the main window meanwhile, the tab included.
fn tab_ghost_over_a_float(scale: f32) -> Rect {
    let mut multi = Multi::new();
    let mut app = app();
    app.viewer.preview = true;
    multi.passes(3, frame(&mut app));
    let at = if scale > 1.0 {
        pos2(2100.0, 200.0)
    } else {
        pos2(700.0, 100.0)
    };
    float(&mut multi, &mut app, "C", at, scale);
    let float = viewport("float-1");

    let b = tab_rect(&app, "B").center();
    let over = float_middle(at, scale);
    multi.press(ViewportId::ROOT, b, frame(&mut app));
    multi.move_held(ViewportId::ROOT, b, over, frame(&mut app));
    let preview = PREVIEW.gamma_multiply(0.7);
    assert!(
        texts(&multi, float).iter().any(|t| t == "B"),
        "scale {scale}: the tab's ghost is painted in the float under the pointer"
    );
    let bodies = filled(&multi, float, preview);
    assert_eq!(bodies.len(), 1, "scale {scale}: the viewer's body, once");
    assert!(
        !texts(&multi, ViewportId::ROOT).iter().any(|t| t == "B"),
        "scale {scale}: the main window paints neither the ghost nor the tab in its bar"
    );
    assert!(
        filled(&multi, ViewportId::ROOT, preview).is_empty(),
        "scale {scale}: no ghost body in the main window"
    );
    multi.release(ViewportId::ROOT, over, frame(&mut app));
    multi.passes(3, frame(&mut app));
    assert_eq!(sorted(tabs(&app, "float-1")), ["B", "C"]);
    assert!(
        filled(&multi, float, preview).is_empty(),
        "no ghost once dropped"
    );
    bodies[0]
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_dragged_over_a_float_shows_its_ghost_there_at_scales_1_and_2() {
    let one = tab_ghost_over_a_float(1.0);
    let two = tab_ghost_over_a_float(2.0);
    assert_eq!(one.size(), two.size(), "the same size in points");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_vetoed_drop_into_a_float_keeps_both_sides() {
    let mut multi = Multi::new();
    let mut app = app();
    app.viewer.refuse_floats = true;
    let at = with_float(&mut multi, &mut app);

    let b = tab_rect(&app, "B").center();
    multi.drag(b, float_middle(at, 1.0), frame(&mut app));

    assert_eq!(tabs(&app, "main"), ["A", "B"]);
    assert_eq!(tabs(&app, "float-1"), ["C"]);
    assert!(app.docked.layout.float("float-2").is_none(), "no new float");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn escape_cancels_a_drag_across_windows() {
    let mut multi = Multi::new();
    let mut app = app();
    let at = with_float(&mut multi, &mut app);

    let b = tab_rect(&app, "B").center();
    let over = float_middle(at, 1.0);
    multi.press(ViewportId::ROOT, b, frame(&mut app));
    multi.move_held(ViewportId::ROOT, b, over, frame(&mut app));
    multi.queue(
        ViewportId::ROOT,
        Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
    );
    multi.passes(2, frame(&mut app));
    multi.release(ViewportId::ROOT, over, frame(&mut app));
    multi.passes(3, frame(&mut app));

    assert_eq!(tabs(&app, "main"), ["A", "B"]);
    assert_eq!(tabs(&app, "float-1"), ["C"]);
    assert!(app.docked.layout.float("float-2").is_none());
    assert!(app.docked.space.dragging().is_none());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_focus_lost_cancels_a_drag_across_windows() {
    // On Wayland, a screenshot taken during the drag grabs the pointer: the button is let go
    // while the compositor holds it, and the drag's window only loses the focus.
    let mut multi = Multi::new();
    let mut app = app();
    let at = with_float(&mut multi, &mut app);
    let float = viewport("float-1");

    let b = tab_rect(&app, "B").center();
    let over = float_middle(at, 1.0);
    multi.press(ViewportId::ROOT, b, frame(&mut app));
    multi.move_held(ViewportId::ROOT, b, over, frame(&mut app));
    assert!(app.docked.space.dragging().is_some());
    assert!(
        texts(&multi, float).iter().any(|text| text == "B"),
        "the ghost is shown over the float"
    );

    multi.queue(ViewportId::ROOT, Event::WindowFocused(false));
    multi.passes(3, frame(&mut app));
    assert!(
        app.docked.space.dragging().is_none(),
        "the drag is cancelled"
    );
    assert!(
        !texts(&multi, float).iter().any(|text| text == "B"),
        "no ghost is left over the float"
    );

    // Back in the window, the pointer moves on, egui still believing the button held, and the
    // next release comes: neither brings the drag back nor drops the tab.
    multi.queue(ViewportId::ROOT, Event::WindowFocused(true));
    multi.move_held(
        ViewportId::ROOT,
        over,
        over + vec2(10.0, 10.0),
        frame(&mut app),
    );
    assert!(app.docked.space.dragging().is_none());
    multi.release(ViewportId::ROOT, over, frame(&mut app));
    multi.passes(3, frame(&mut app));

    assert_eq!(tabs(&app, "main"), ["A", "B"]);
    assert_eq!(tabs(&app, "float-1"), ["C"]);
    assert!(app.docked.layout.float("float-2").is_none());
    assert!(app.docked.space.dragging().is_none());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn released_over_nothing_a_tab_opens_a_new_float_there() {
    let mut multi = Multi::new();
    let mut app = app();
    with_float(&mut multi, &mut app);

    let b = tab_rect(&app, "B").center();
    let nowhere = pos2(1200.0, 700.0);
    multi.drag(b, nowhere, frame(&mut app));

    let float = app.docked.layout.float("float-2").expect("a new float");
    assert_eq!(tabs(&app, "float-2"), ["B"]);
    assert_eq!(float.place.screen.as_deref(), Some("one"));
    assert_eq!(float.place.rect.min, nowhere, "where released");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn released_over_nothing_and_refused_a_tab_goes_back() {
    let mut multi = Multi::new();
    let mut app = app();
    app.viewer.refuse_windows = true;
    with_float(&mut multi, &mut app);

    let b = tab_rect(&app, "B").center();
    multi.drag(b, pos2(1200.0, 700.0), frame(&mut app));

    assert!(app.docked.layout.float("float-2").is_none());
    assert_eq!(tabs(&app, "main"), ["A", "B"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_torn_off_a_native_float_opens_a_float_of_its_own() {
    let mut multi = Multi::new();
    let mut app = app();
    let at = with_float(&mut multi, &mut app);
    // `B` joins `C` in the float first.
    let b = tab_rect(&app, "B").center();
    multi.drag(b, float_middle(at, 1.0), frame(&mut app));
    assert_eq!(sorted(tabs(&app, "float-1")), ["B", "C"]);

    let c = tab_rect(&app, "C").center();
    let nowhere = pos2(1300.0, 600.0);
    multi.drag_in(
        viewport("float-1"),
        c,
        local(nowhere, at, 1.0),
        frame(&mut app),
    );

    assert_eq!(tabs(&app, "float-1"), ["B"]);
    let key = app
        .docked
        .layout
        .floats
        .iter()
        .find(|f| f.key != "float-1")
        .map(|f| f.key.clone())
        .expect("a new float");
    assert_eq!(tabs(&app, &key), ["C"]);
    let place = &app.docked.layout.float(&key).unwrap().place;
    assert_eq!(place.screen.as_deref(), Some("one"));
    assert_eq!(place.rect.min, nowhere, "where released");
    assert_eq!(
        app.docked
            .space
            .origin(Id::unique("C"))
            .map(|o| o.dock.as_str()),
        Some("main"),
        "torn off a float, its origin kept"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_target_window_closed_during_the_drag_cancels_it() {
    let mut multi = Multi::new();
    let mut app = app();
    let at = with_float(&mut multi, &mut app);

    let b = tab_rect(&app, "B").center();
    let over = float_middle(at, 1.0);
    multi.press(ViewportId::ROOT, b, frame(&mut app));
    multi.move_held(ViewportId::ROOT, b, over, frame(&mut app));
    multi.request_close(viewport("float-1"));
    multi.passes(2, frame(&mut app));
    assert!(app.docked.layout.float("float-1").is_none(), "closed");
    assert!(app.docked.space.dragging().is_some_and(|d| d.cancelled));
    multi.release(ViewportId::ROOT, over, frame(&mut app));
    multi.passes(3, frame(&mut app));

    // Nothing lost, nothing duplicated: `C` went back, `B` stayed.
    assert_eq!(tabs(&app, "main"), ["A", "B", "C"]);
    assert!(app.docked.layout.floats.is_empty(), "no new float");
    assert!(app.docked.space.dragging().is_none());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn where_positions_are_hidden_the_window_the_pointer_enters_takes_the_drop() {
    let mut multi = Multi::new();
    multi.hide_positions();
    let mut app = app();
    with_float(&mut multi, &mut app);

    let b = tab_rect(&app, "B").center();
    // Past the main window's right edge: no window can tell whether it is over it.
    let past = pos2(900.0, 200.0);
    multi.press(ViewportId::ROOT, b, frame(&mut app));
    multi.move_held(ViewportId::ROOT, b, past, frame(&mut app));
    assert_eq!(app.docked.space.dragging().and_then(|d| d.over), None);
    multi.release(ViewportId::ROOT, past, frame(&mut app));
    assert!(
        app.docked.space.dragging().is_some_and(|d| d.released),
        "waits for a window to see the pointer"
    );
    // The pointer comes into the float's window.
    let middle = (FLOAT / 2.0).to_pos2();
    multi.queue(viewport("float-1"), Event::PointerMoved(middle));
    multi.passes(4, frame(&mut app));

    assert_eq!(sorted(tabs(&app, "float-1")), ["B", "C"]);
    assert_eq!(tabs(&app, "main"), ["A"]);
    assert!(app.docked.space.dragging().is_none());
    assert!(app.docked.layout.float("float-2").is_none());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn where_positions_are_hidden_and_no_window_takes_it_a_new_float_opens() {
    let mut multi = Multi::new();
    multi.hide_positions();
    let mut app = app();
    with_float(&mut multi, &mut app);

    let b = tab_rect(&app, "B").center();
    multi.drag(b, pos2(900.0, 200.0), frame(&mut app));
    multi.passes(20, frame(&mut app));

    assert_eq!(tabs(&app, "float-2"), ["B"]);
    assert_eq!(
        app.docked.layout.float("float-2").unwrap().place.screen,
        None,
        "no position invented"
    );
    assert_eq!(tabs(&app, "float-1"), ["C"]);
    assert!(app.docked.space.dragging().is_none());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn tabs_cross_screens_of_scale_1_and_2() {
    let mut multi = Multi::new();
    let mut app = app();
    multi.passes(3, frame(&mut app));
    // `C` in a float on the screen of scale 2.
    let at = pos2(2100.0, 200.0);
    float(&mut multi, &mut app, "C", at, 2.0);
    assert_eq!(
        app.docked
            .layout
            .float("float-1")
            .unwrap()
            .place
            .screen
            .as_deref(),
        Some("two")
    );

    // From the main window (scale 1) into it.
    let b = tab_rect(&app, "B").center();
    multi.drag(b, float_middle(at, 2.0), frame(&mut app));
    assert_eq!(sorted(tabs(&app, "float-1")), ["B", "C"]);

    // And from it back into the side dock.
    let c = tab_rect(&app, "C").center();
    multi.drag_in(
        viewport("float-1"),
        c,
        local(side_middle(), at, 2.0),
        frame(&mut app),
    );
    assert_eq!(sorted(tabs(&app, "side")), ["C", "S"]);
    assert_eq!(tabs(&app, "float-1"), ["B"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_moved_to_a_screen_of_another_scale_during_a_drag_takes_the_drop_without_resizing() {
    let mut multi = Multi::new();
    let mut app = app();
    let at = with_float(&mut multi, &mut app);
    let first = multi.builder(viewport("float-1")).unwrap().clone();
    multi.take_commands(viewport("float-1"));

    let b = tab_rect(&app, "B").center();
    multi.press(ViewportId::ROOT, b, frame(&mut app));
    multi.move_held(ViewportId::ROOT, b, float_middle(at, 1.0), frame(&mut app));
    // Mid-drag, the window goes to the screen of scale 2: its size in points is kept.
    let moved = pos2(2100.0, 200.0);
    multi.place(viewport("float-1"), moved, 2.0);
    multi.passes(5, frame(&mut app));
    let over = float_middle(moved, 2.0);
    multi.move_held(
        ViewportId::ROOT,
        float_middle(at, 1.0),
        over,
        frame(&mut app),
    );
    assert_eq!(
        app.docked.space.dragging().and_then(|d| d.over),
        Some(viewport("float-1"))
    );
    multi.release(ViewportId::ROOT, over, frame(&mut app));
    multi.passes(3, frame(&mut app));

    assert_eq!(
        sorted(tabs(&app, "float-1")),
        ["B", "C"],
        "lands where released"
    );
    let float = app.docked.layout.float("float-1").unwrap();
    assert_eq!(float.place.screen.as_deref(), Some("two"));
    assert_eq!(float.place.rect.size(), FLOAT, "not resized");
    // No resize loop: the window is never told to move or resize.
    assert_eq!(multi.last_builder(viewport("float-1")), Some(&first));
    let commands = multi.take_commands(viewport("float-1"));
    assert!(
        !commands.iter().any(|command| matches!(
            command,
            ViewportCommand::InnerSize(_) | ViewportCommand::OuterPosition(_)
        )),
        "{commands:?}"
    );
    let moves = app
        .docked
        .space
        .take_float_events()
        .into_iter()
        .filter(|event| matches!(event, FloatEvent::Moved { .. }))
        .count();
    assert_eq!(moves, 1, "its move said once");
}
