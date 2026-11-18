//! Floats drawn by `FloatsArea`, on a harness whose viewports are real (`support/multi.rs`):
//! a tab dragged out of every dock opens a float (native or over the main window), closing it
//! applies the close policy, and a screen that goes moves or docks back its floats.

#[path = "support/multi.rs"]
mod multi;

use egui::{Id, Pos2, Rect, Ui, UiBuilder, ViewportCommand, ViewportId, WidgetText, pos2, vec2};
use egui_stack::layout::{Content, Dir, FloatHost, Layout, Screen, ScreenGone, Zone};
use egui_stack::{
    ClosePolicy, DockArea, DockRules, DockSpace, Docked, FloatClose, FloatEvent, NodeIndex,
    TabOrigin, TabPath, TabViewer, ViewportHost,
};
use multi::Multi;

/// Where the main dock is drawn; the side dock is on its right. Below 300 points is outside
/// every dock.
const MAIN: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(300.0, 300.0));
const SIDE: Rect = Rect::from_min_max(pos2(300.0, 0.0), pos2(600.0, 300.0));
const OUTSIDE: Pos2 = pos2(320.0, 420.0);

#[derive(Default)]
struct Viewer {
    tab_rects: Vec<(String, Rect)>,
    policy: ClosePolicy,
    asked: Vec<FloatClose>,
    /// Each tab drawn, and the viewport it was drawn in.
    drawn_in: Vec<(String, ViewportId)>,
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
        self.drawn_in.push((tab.clone(), ui.ctx().viewport_id()));
        ui.label(tab.as_str());
    }

    fn on_tab_button(&mut self, tab: &mut String, response: &egui::Response) {
        self.tab_rects.push((tab.clone(), response.rect));
    }

    fn close_policy(&mut self, close: &FloatClose) -> ClosePolicy {
        self.asked.push(close.clone());
        self.policy.clone()
    }
}

struct Host {
    screens: Vec<Screen>,
    /// The application's own viewport ids, instead of egui_stack's default ones.
    own_ids: bool,
}

impl ViewportHost for Host {
    fn viewport_id(&self, key: &str) -> ViewportId {
        if self.own_ids {
            ViewportId::from_hash_of(("app.window", key))
        } else {
            ViewportId::from_hash_of(("egui_stack.float", key))
        }
    }

    fn screens(&self) -> Vec<Screen> {
        self.screens.clone()
    }
}

struct App {
    docked: Docked<String>,
    viewer: Viewer,
    host: Host,
    gone: ScreenGone,
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

fn strings(tabs: &[&str]) -> Vec<String> {
    tabs.iter().map(|tab| (*tab).to_owned()).collect()
}

/// `A`, `B`, `C` in the main dock, `S` in the side dock; floats hosted by `host`.
fn app(host: FloatHost) -> App {
    let mut space = DockSpace::new().with_float_host(host);
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
        host: Host {
            screens: vec![one(), two()],
            own_ids: false,
        },
        gone: ScreenGone::MoveOnto,
    }
}

/// One frame of the application: the two docks, then the floats.
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
            .host(&mut app.host)
            .screen_gone(app.gone)
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

fn viewport(key: &str) -> ViewportId {
    Host {
        screens: vec![],
        own_ids: false,
    }
    .viewport_id(key)
}

/// Starts the application and drags `B` out of every dock: its float, `float-1`.
fn detach_b(multi: &mut Multi, app: &mut App) {
    multi.passes(3, frame(app));
    let b = tab_rect(app, "B").center();
    multi.drag(b, OUTSIDE, frame(app));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_released_outside_every_dock_opens_a_native_float() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    detach_b(&mut multi, &mut app);

    let float = app.docked.layout.float("float-1").expect("a float");
    assert_eq!(float.host, FloatHost::Native);
    assert_eq!(float.dock_name(), Some("float-1"));
    assert_eq!(float.place.screen.as_deref(), Some("one"));
    assert_eq!(tabs(&app, "float-1"), ["B"]);
    assert_eq!(tabs(&app, "main"), ["A", "C"]);
    assert!(
        app.docked
            .space
            .state()
            .iter_surfaces()
            .all(|s| !matches!(s, egui_stack::Surface::Window(..))),
        "no legacy window"
    );
    assert_eq!(
        app.docked.space.origin(Id::unique("B")),
        Some(&TabOrigin::new(
            Id::unique("B"),
            "main",
            Some(Id::unique("A")),
            Some(Id::unique("C"))
        ))
    );
    assert!(multi.shown().contains(&viewport("float-1")), "its window");
    let builder = multi.builder(viewport("float-1")).unwrap();
    assert!(builder.inner_size.is_some(), "{builder:?}");
    assert!(builder.position.is_some(), "on a known screen: {builder:?}");
    let events = app.docked.space.take_float_events();
    assert!(
        matches!(&events[..], [FloatEvent::Detached { key, tabs, host: FloatHost::Native, .. }]
            if key == "float-1" && tabs == &[Id::unique("B")]),
        "{events:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn closed_a_native_float_returns_its_tab_where_it_came_from() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    detach_b(&mut multi, &mut app);
    app.docked.space.take_float_events();

    multi.request_close(viewport("float-1"));
    multi.passes(2, frame(&mut app));

    assert!(app.docked.layout.float("float-1").is_none());
    assert!(
        app.docked.layout.closed_place("float-1").is_some(),
        "its place kept"
    );
    assert!(!app.docked.space.has_dock("float-1"));
    assert_eq!(tabs(&app, "main"), ["A", "B", "C"]);
    assert_eq!(app.docked.space.origin(Id::unique("B")), None);
    assert!(
        !multi.shown().contains(&viewport("float-1")),
        "its window is gone"
    );
    assert_eq!(app.viewer.asked.len(), 1);
    assert_eq!(app.viewer.asked[0].tabs, [Id::unique("B")]);
    let events = app.docked.space.take_float_events();
    assert!(
        matches!(&events[..], [FloatEvent::Closed { key, policy: Some(ClosePolicy::ReturnToOrigin), moved, .. }]
            if key == "float-1" && moved == &[(Id::unique("B"), "main".to_owned())]),
        "{events:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn closed_into_a_dock_its_tabs_go_there() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    app.viewer.policy = ClosePolicy::Into("side".to_owned());
    detach_b(&mut multi, &mut app);

    multi.request_close(viewport("float-1"));
    multi.passes(2, frame(&mut app));

    assert!(app.docked.layout.float("float-1").is_none());
    assert_eq!(tabs(&app, "side"), ["S", "B"]);
    assert_eq!(tabs(&app, "main"), ["A", "C"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn kept_a_float_stays_open_and_its_close_is_cancelled() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    app.viewer.policy = ClosePolicy::Keep;
    detach_b(&mut multi, &mut app);
    multi.take_commands(viewport("float-1"));

    multi.request_close(viewport("float-1"));
    multi.passes(2, frame(&mut app));

    assert!(app.docked.layout.float("float-1").is_some());
    assert_eq!(tabs(&app, "float-1"), ["B"]);
    assert!(multi.shown().contains(&viewport("float-1")));
    let commands = multi.take_commands(viewport("float-1"));
    assert!(
        commands.contains(&ViewportCommand::CancelClose),
        "{commands:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn closed_with_its_tabs_they_are_gone() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    app.viewer.policy = ClosePolicy::CloseTabs;
    detach_b(&mut multi, &mut app);

    multi.request_close(viewport("float-1"));
    multi.passes(2, frame(&mut app));

    assert!(app.docked.layout.float("float-1").is_none());
    assert_eq!(tabs(&app, "main"), ["A", "C"]);
    assert!(app.docked.space.state().find_tab(&"B".to_owned()).is_none());
}

/// Opens `B` in a float on the screen `two`, as a layout saved there would.
fn float_on_two(multi: &mut Multi, app: &mut App) {
    multi.passes(2, frame(app));
    let main = app.docked.space.dock_surface("main").unwrap();
    let key = app
        .docked
        .space
        .float_out(
            TabPath::new(main, NodeIndex::root(), egui_stack::TabIndex(1)),
            Rect::from_min_size(pos2(100.0, 100.0), vec2(300.0, 200.0)),
            |tab| Id::unique(tab.as_str()),
        )
        .unwrap();
    multi.pass(frame(app));
    // The user moves its window onto the second screen.
    multi.set_outer_position(viewport(&key), pos2(2100.0, 200.0));
    multi.passes(3, frame(app));
    let place = &app.docked.layout.float(&key).unwrap().place;
    assert_eq!(place.screen.as_deref(), Some("two"));
    assert_eq!(
        place.rect.min,
        pos2(90.0, 100.0),
        "points on a screen of scale 2"
    );
    app.docked.space.take_float_events();
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_whose_screen_goes_moves_onto_another() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    float_on_two(&mut multi, &mut app);

    app.host.screens = vec![one()];
    multi.passes(2, frame(&mut app));

    let float = app.docked.layout.float("float-1").expect("still open");
    assert_eq!(float.place.screen.as_deref(), Some("one"));
    assert!(one().local_rect().contains_rect(float.place.rect));
    let events = app.docked.space.take_float_events();
    assert!(
        events.iter().any(
            |event| matches!(event, FloatEvent::Repaired { key, from, to }
            if key == "float-1" && from.as_deref() == Some("two") && to.as_deref() == Some("one"))
        ),
        "{events:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_whose_screen_goes_docks_back_when_asked() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    app.gone = ScreenGone::DockBack;
    float_on_two(&mut multi, &mut app);

    app.host.screens = vec![one()];
    multi.passes(2, frame(&mut app));

    assert!(app.docked.layout.float("float-1").is_none());
    assert_eq!(tabs(&app, "main"), ["A", "B", "C"]);
    assert!(!multi.shown().contains(&viewport("float-1")));
    let events = app.docked.space.take_float_events();
    assert!(
        matches!(
            &events[..],
            [
                FloatEvent::Repaired { to: None, .. },
                FloatEvent::Closed {
                    policy: Some(ClosePolicy::ReturnToOrigin),
                    ..
                },
            ]
        ),
        "{events:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_moved_by_the_user_keeps_its_place_and_says_so_once() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    detach_b(&mut multi, &mut app);
    app.docked.space.take_float_events();

    multi.set_outer_position(viewport("float-1"), pos2(700.0, 200.0));
    multi.passes(4, frame(&mut app));

    let place = &app.docked.layout.float("float-1").unwrap().place;
    assert_eq!(place.screen.as_deref(), Some("one"));
    assert_eq!(place.rect.min, pos2(700.0, 200.0));
    let events = app.docked.space.take_float_events();
    assert!(
        matches!(&events[..], [FloatEvent::Moved { key, .. }] if key == "float-1"),
        "{events:?}"
    );
    // Shown again unchanged, the builder does not move the window back.
    let builder = multi.builder(viewport("float-1")).unwrap().clone();
    multi.passes(2, frame(&mut app));
    assert_eq!(multi.builder(viewport("float-1")), Some(&builder));
    assert!(app.docked.space.take_float_events().is_empty());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn where_positions_are_hidden_no_position_is_invented() {
    let mut multi = Multi::new();
    multi.hide_positions();
    let mut app = app(FloatHost::Native);
    detach_b(&mut multi, &mut app);

    let float = app.docked.layout.float("float-1").expect("a float");
    assert_eq!(float.place.screen, None);
    let builder = multi.builder(viewport("float-1")).unwrap();
    assert_eq!(builder.position, None, "{builder:?}");
    assert!(builder.inner_size.is_some());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_whose_dock_empties_closes() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    detach_b(&mut multi, &mut app);
    app.docked.space.take_float_events();

    app.docked
        .space
        .dock_mut("float-1")
        .unwrap()
        .retain_tabs(|_| false);
    multi.passes(3, frame(&mut app));

    assert!(app.docked.layout.float("float-1").is_none());
    assert!(!app.docked.space.has_dock("float-1"));
    let events = app.docked.space.take_float_events();
    assert!(
        matches!(&events[..], [FloatEvent::Closed { policy: None, .. }]),
        "{events:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_legacy_window_becomes_a_float() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    app.docked.space.state_mut().add_window(strings(&["W"]));
    multi.passes(2, frame(&mut app));

    let float = app.docked.layout.float("window-1").expect("a float");
    assert_eq!(float.host, FloatHost::Native);
    assert_eq!(tabs(&app, "window-1"), ["W"]);
    assert!(multi.shown().contains(&viewport("window-1")));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_over_the_main_window_takes_tabs_out_and_back() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::InWindow);
    detach_b(&mut multi, &mut app);

    let float = app.docked.layout.float("float-1").expect("a float");
    assert_eq!(float.host, FloatHost::InWindow);
    assert!(multi.shown().is_empty(), "no native window");
    assert_eq!(tabs(&app, "float-1"), ["B"]);

    // Back into the side dock: the float, emptied, closes.
    let b = tab_rect(&app, "B").center();
    multi.drag(b, pos2(SIDE.center().x, 100.0), frame(&mut app));
    let mut side = tabs(&app, "side");
    side.sort();
    assert_eq!(side, ["B", "S"]);
    multi.passes(2, frame(&mut app));
    assert!(app.docked.layout.float("float-1").is_none());
    let events = app.docked.space.take_float_events();
    assert!(
        // Moved too: egui keeps the window on the screen.
        matches!(
            &events[..],
            [
                FloatEvent::Detached { .. },
                ..,
                FloatEvent::Closed { policy: None, .. }
            ]
        ),
        "{events:?}"
    );
}

/// The viewports `tab` was drawn in since the viewer's list was last cleared.
fn drawn_in(app: &App, tab: &str) -> Vec<ViewportId> {
    app.viewer
        .drawn_in
        .iter()
        .filter(|(name, _)| name == tab)
        .map(|(_, viewport)| *viewport)
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_float_lives_in_the_application_s_own_viewport_and_keeps_it() {
    let mut multi = Multi::new();
    let mut app = app(FloatHost::Native);
    app.host.own_ids = true;
    detach_b(&mut multi, &mut app);
    let own = app.host.viewport_id("float-1");
    assert_ne!(own, viewport("float-1"));
    assert!(multi.shown().contains(&own), "its window");
    assert!(!multi.shown().contains(&viewport("float-1")));

    // The tab is drawn in that viewport, where the application renders what it pairs with it
    // (a wgpu surface, display settings).
    app.viewer.drawn_in.clear();
    multi.passes(2, frame(&mut app));
    let drawn = drawn_in(&app, "B");
    assert!(
        !drawn.is_empty() && drawn.iter().all(|v| *v == own),
        "{drawn:?}"
    );

    // Moved, the window keeps its viewport.
    multi.set_outer_position(own, pos2(700.0, 200.0));
    multi.passes(4, frame(&mut app));
    assert_eq!(
        app.docked.layout.float("float-1").unwrap().place.rect.min,
        pos2(700.0, 200.0)
    );
    app.viewer.drawn_in.clear();
    multi.pass(frame(&mut app));
    let drawn = drawn_in(&app, "B");
    assert!(
        !drawn.is_empty() && drawn.iter().all(|v| *v == own),
        "{drawn:?}"
    );

    // Closed through it, the tab comes back to the main window.
    multi.request_close(own);
    multi.passes(2, frame(&mut app));
    assert!(app.docked.layout.float("float-1").is_none());
    assert!(!multi.shown().contains(&own));
    assert_eq!(tabs(&app, "main"), ["A", "B", "C"]);
    app.viewer.drawn_in.clear();
    multi.pass(frame(&mut app));
    assert!(drawn_in(&app, "B").iter().all(|v| *v == ViewportId::ROOT));
}
