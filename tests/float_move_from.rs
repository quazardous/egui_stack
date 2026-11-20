//! Moving a float over the main window from elsewhere than its bar (`FloatsArea::move_from`):
//! a drag on its empty space, a drag with a modifier held over its widgets. By default only
//! the move handle moves it. The group's menu opens in a float too, on a right click on the
//! empty part of its tab bar. On the multi-viewport harness (`support/multi.rs`).

#[path = "support/multi.rs"]
mod multi;

use egui::{
    Event, Id, Modifiers, PointerButton, Pos2, Rect, Ui, UiBuilder, ViewportId, WidgetText, pos2,
    vec2,
};
use egui_stack::layout::{Content, Dir, FloatHost, Layout, Zone};
use egui_stack::{DockArea, DockRules, DockSpace, Docked, MoveFrom, TabViewer, ViewportHost};
use multi::Multi;

const MAIN: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(300.0, 300.0));
const SIDE: Rect = Rect::from_min_max(pos2(300.0, 0.0), pos2(600.0, 300.0));
const OUTSIDE: Pos2 = pos2(320.0, 420.0);
const SLIDER: &str = "Level";

#[derive(Default)]
struct Viewer {
    tab_rects: Vec<(String, Rect)>,
    level: f32,
    /// Where the slider was last drawn.
    slider: Option<Rect>,
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
        if tab == "B" {
            let response = ui.add(egui::Slider::new(&mut self.level, 0.0..=100.0).text(SLIDER));
            self.slider = Some(response.rect);
        } else {
            ui.label(tab.as_str());
        }
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
    move_from: MoveFrom,
}

fn app(move_from: MoveFrom) -> App {
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
        viewer: Viewer::default(),
        move_from,
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
            .move_from(app.move_from)
            .show(ui.ctx(), &mut app.viewer);
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

fn float_rect(app: &App) -> Rect {
    app.docked.layout.float("float-1").unwrap().place.rect
}

/// A point of the float's body no widget covers: below the slider.
fn empty_body(app: &App) -> Pos2 {
    let slider = app.viewer.slider.expect("the slider is shown");
    pos2(slider.left() + 10.0, slider.bottom() + 30.0)
}

/// A point on the slider's rail.
fn slider(app: &App) -> Pos2 {
    let rect = app.viewer.slider.expect("the slider is shown");
    pos2(rect.left() + 20.0, rect.center().y)
}

/// Drags from `at` by `by` and returns how far the float moved.
fn drag_by(multi: &mut Multi, app: &mut App, at: fn(&App) -> Pos2, by: egui::Vec2) -> egui::Vec2 {
    let at = at(app);
    let before = float_rect(app);
    multi.drag(at, at + by, frame(app));
    float_rect(app).min - before.min
}

const BY: egui::Vec2 = vec2(-60.0, -40.0);

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn by_default_the_empty_space_and_alt_do_not_move_the_float() {
    let mut multi = Multi::new();
    let mut app = app(MoveFrom::default());
    detach_b(&mut multi, &mut app);
    let moved = drag_by(&mut multi, &mut app, empty_body, BY);
    assert!(moved.length() < 1.0, "{moved:?}");

    multi.hold(ViewportId::ROOT, Modifiers::ALT);
    multi.pass(frame(&mut app));
    let moved = drag_by(&mut multi, &mut app, slider, vec2(30.0, 0.0));
    assert!(moved.length() < 1.0, "{moved:?}");
    assert!(app.viewer.level > 0.0, "the slider took the drag");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_drag_on_empty_space_moves_the_float_a_slider_keeps_its_drag() {
    let mut multi = Multi::new();
    let mut app = app(MoveFrom::EMPTY_SPACE);
    detach_b(&mut multi, &mut app);

    let moved = drag_by(&mut multi, &mut app, empty_body, BY);
    assert!((moved - BY).length() < 2.0, "{moved:?}");

    let moved = drag_by(&mut multi, &mut app, slider, vec2(30.0, 0.0));
    assert!(moved.length() < 1.0, "the slider's drag: {moved:?}");
    assert!(app.viewer.level > 0.0, "the slider moved");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn alt_drag_over_a_widget_moves_the_float() {
    let mut multi = Multi::new();
    let mut app = app(MoveFrom::modifier(Modifiers::ALT));
    detach_b(&mut multi, &mut app);

    // Without Alt, the empty space does not move it.
    let moved = drag_by(&mut multi, &mut app, empty_body, BY);
    assert!(moved.length() < 1.0, "{moved:?}");

    multi.hold(ViewportId::ROOT, Modifiers::ALT);
    multi.pass(frame(&mut app));
    let moved = drag_by(&mut multi, &mut app, slider, BY);
    assert!((moved - BY).length() < 2.0, "{moved:?}");
    assert_eq!(app.viewer.level, 0.0, "the slider did not take the drag");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_right_click_on_a_float_s_empty_bar_opens_the_group_s_menu() {
    let mut multi = Multi::new();
    let mut app = app(MoveFrom::default());
    detach_b(&mut multi, &mut app);
    let b = app
        .viewer
        .tab_rects
        .iter()
        .find(|(name, _)| name == "B")
        .map(|(_, rect)| *rect)
        .unwrap();
    let at = pos2(b.right() + 20.0, b.center().y);
    multi.queue(ViewportId::ROOT, Event::PointerMoved(at));
    multi.pass(frame(&mut app));
    for pressed in [true, false] {
        multi.queue(
            ViewportId::ROOT,
            Event::PointerButton {
                pos: at,
                button: PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            },
        );
        multi.pass(frame(&mut app));
    }
    multi.passes(2, frame(&mut app));
    let labels: Vec<String> = multi
        .labels(ViewportId::ROOT)
        .into_iter()
        .map(|(label, _)| label)
        .collect();
    assert!(
        labels.iter().any(|l| l == "Close all tabs"),
        "the group's menu: {labels:?}"
    );
    // Alone in its float: no Eject group, nor a new window to move to.
    assert!(!labels.iter().any(|l| l == "Eject group"), "{labels:?}");
}
