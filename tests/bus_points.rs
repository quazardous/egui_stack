//! egui_stack's points on an application's bus (the `bus` feature): the drop veto is asked
//! through `stack.dock.drop` (fail-closed, asked when the destination changes and at release,
//! never once per frame), the close policy through `stack.window.close` (fails open to the
//! default), and the window events are emitted.

#![cfg(feature = "bus")]

#[path = "support/multi.rs"]
mod multi;
#[path = "support/test_bus.rs"]
mod test_bus;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use egui::{
    Event, Id, Modifiers, PointerButton, Pos2, Rect, Ui, UiBuilder, ViewportId, WidgetText, pos2,
};
use egui_stack::bus::bus_contract::{Bus, HookError, ListenOpts};
use egui_stack::bus::{self, ClosePolicy, POINTS};
use egui_stack::layout::{Content, Dir, FloatHost, Layout, Screen, Zone};
use egui_stack::{DockArea, DockRules, DockSpace, Docked, FloatClose, TabViewer, ViewportHost};
use multi::Multi;
use test_bus::TestBus;

const MAIN: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(300.0, 300.0));
const SIDE: Rect = Rect::from_min_max(pos2(300.0, 0.0), pos2(600.0, 300.0));
const OUTSIDE: Pos2 = pos2(320.0, 420.0);

#[derive(Default)]
struct Viewer {
    tab_rects: Vec<(String, Rect)>,
    policy: ClosePolicy,
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

    fn close_policy(&mut self, _close: &FloatClose) -> ClosePolicy {
        self.policy.clone()
    }

    fn dock_drop_allowed(
        &self,
        _tab: &String,
        _drop: &egui_stack::DockDrop<'_>,
    ) -> egui_stack::tab_viewer::DropVerdict {
        // Refuses everything: a drop that lands shows the viewer was not asked.
        egui_stack::tab_viewer::DropVerdict::Refuse(None)
    }
}

struct Host;

impl ViewportHost for Host {
    fn screens(&self) -> Vec<Screen> {
        vec![Screen::new(
            "one",
            Rect::from_min_size(Pos2::ZERO, egui::vec2(1920.0, 1080.0)),
            1.0,
        )]
    }
}

struct App {
    docked: Docked<String>,
    viewer: Viewer,
}

fn app(bus: Arc<TestBus>) -> App {
    let mut space = DockSpace::new()
        .with_float_host(FloatHost::Native)
        .with_bus(bus, |tab: &String| tab.clone());
    space.add_dock(
        "main",
        DockRules::default(),
        vec!["A".to_owned(), "B".to_owned(), "C".to_owned()],
    );
    space.add_dock("side", DockRules::default(), vec!["S".to_owned()]);
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
        app.viewer.tab_rects.clear();
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

fn viewport(key: &str) -> ViewportId {
    Host.viewport_id(key)
}

/// Counts the asks of `stack.dock.drop`, refusing those `refuse` says.
fn count_drops(
    bus: &TestBus,
    refuse: impl Fn(&bus::DropAsk) -> Option<HookError> + Send + Sync + 'static,
) -> Arc<AtomicUsize> {
    let asked = Arc::new(AtomicUsize::new(0));
    let count = asked.clone();
    let bus: &dyn Bus = bus;
    bus.listen::<bus::DockDrop>("test.veto", ListenOpts::new(), move |ask, _| {
        count.fetch_add(1, Ordering::SeqCst);
        refuse(ask).map_or(Ok(()), Err)
    })
    .unwrap();
    asked
}

/// The middle of the side dock's leaf, below its tab bar.
fn side_middle() -> Pos2 {
    pos2(SIDE.center().x, 150.0)
}

#[test]
fn the_points_are_declared_under_the_crates_prefix() {
    let bus = Arc::new(TestBus::default());
    let _app = app(bus.clone());
    let mut declared = bus.declared();
    declared.sort_unstable();
    let mut expected: Vec<&str> = POINTS.iter().map(|point| point.id).collect();
    expected.sort_unstable();
    assert_eq!(declared, expected);
    assert!(declared.iter().all(|id| id.starts_with("stack.")));
    assert!(
        POINTS
            .iter()
            .find(|p| p.id == "stack.dock.drop")
            .unwrap()
            .fail_closed
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_veto_listener_refuses_a_drop() {
    let bus = Arc::new(TestBus::default());
    let asked = count_drops(&bus, |ask| {
        (ask.to.as_deref() == Some("side")).then(|| HookError::refuse("not there"))
    });
    let mut multi = Multi::new();
    let mut app = app(bus);
    multi.passes(3, frame(&mut app));
    let a = tab_rect(&app, "A").center();
    multi.drag(a, side_middle(), frame(&mut app));

    assert_eq!(tabs(&app, "side"), ["S"]);
    assert_eq!(tabs(&app, "main"), ["A", "B", "C"]);
    assert!(asked.load(Ordering::SeqCst) > 0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_failing_drop_listener_refuses_the_drop() {
    let bus = Arc::new(TestBus::default());
    count_drops(&bus, |_| Some(HookError::failed("broken")));
    let mut multi = Multi::new();
    let mut app = app(bus);
    multi.passes(3, frame(&mut app));
    let a = tab_rect(&app, "A").center();
    multi.drag(a, side_middle(), frame(&mut app));

    assert_eq!(tabs(&app, "side"), ["S"], "fail-closed");
    assert_eq!(tabs(&app, "main"), ["A", "B", "C"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn an_allowed_drop_lands_and_the_viewer_is_not_asked() {
    let bus = Arc::new(TestBus::default());
    count_drops(&bus, |_| None);
    let mut multi = Multi::new();
    let mut app = app(bus);
    multi.passes(3, frame(&mut app));
    let a = tab_rect(&app, "A").center();
    multi.drag(a, side_middle(), frame(&mut app));

    // The viewer refuses everything: the bus replaced it.
    assert!(tabs(&app, "side").contains(&"A".to_owned()));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_drop_is_asked_when_the_destination_changes_not_every_frame() {
    let bus = Arc::new(TestBus::default());
    let asked = count_drops(&bus, |_| None);
    let mut multi = Multi::new();
    let mut app = app(bus);
    multi.passes(3, frame(&mut app));
    let from = tab_rect(&app, "A").center();
    let to = side_middle();
    let root = ViewportId::ROOT;
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    multi.queue(root, Event::PointerMoved(from));
    multi.pass(frame(&mut app));
    multi.queue(root, button(from, true));
    multi.pass(frame(&mut app));
    for step in 1..=10 {
        let at = from + (to - from) * (step as f32 / 10.0);
        multi.queue(root, Event::PointerMoved(at));
        multi.pass(frame(&mut app));
    }
    for _ in 0..30 {
        multi.queue(root, Event::PointerMoved(to));
        multi.pass(frame(&mut app));
    }
    let held = asked.load(Ordering::SeqCst);
    assert!(held > 0, "asked on the way");
    for _ in 0..30 {
        multi.queue(root, Event::PointerMoved(to));
        multi.pass(frame(&mut app));
    }
    assert_eq!(
        asked.load(Ordering::SeqCst),
        held,
        "not asked again while held"
    );
    multi.queue(root, button(to, false));
    multi.passes(3, frame(&mut app));
    assert_eq!(
        asked.load(Ordering::SeqCst),
        held + 1,
        "asked again at release"
    );
    assert!(tabs(&app, "side").contains(&"A".to_owned()));
}

/// Starts the application and drags `B` out of every dock: its float, `float-1`.
fn detach_b(multi: &mut Multi, app: &mut App) {
    multi.passes(3, frame(app));
    let b = tab_rect(app, "B").center();
    multi.drag(b, OUTSIDE, frame(app));
    assert!(app.docked.layout.float("float-1").is_some(), "detached");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_failing_close_listener_falls_back_to_the_default() {
    let bus = Arc::new(TestBus::default());
    {
        let bus: &dyn Bus = &*bus;
        bus.listen::<bus::WindowClose>("test.close", ListenOpts::new(), |_, _| {
            Err(HookError::failed("broken"))
        })
        .unwrap();
    }
    let mut multi = Multi::new();
    let mut app = app(bus);
    // The viewer would keep the window: a broken listener must not trap it either way.
    app.viewer.policy = ClosePolicy::Keep;
    detach_b(&mut multi, &mut app);

    multi.request_close(viewport("float-1"));
    multi.passes(2, frame(&mut app));

    assert!(app.docked.layout.float("float-1").is_none(), "closed");
    assert_eq!(
        tabs(&app, "main"),
        ["A", "B", "C"],
        "returned to its origin"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_close_listener_chooses_the_policy() {
    let bus = Arc::new(TestBus::default());
    {
        let bus: &dyn Bus = &*bus;
        bus.listen::<bus::WindowClose>("test.close", ListenOpts::new(), |ask, policy| {
            assert_eq!(ask.tabs, ["B"]);
            assert_eq!(ask.origins, [Some("main".to_owned())]);
            *policy = ClosePolicy::Into("side".to_owned());
            Ok(())
        })
        .unwrap();
    }
    let mut multi = Multi::new();
    let mut app = app(bus);
    detach_b(&mut multi, &mut app);

    multi.request_close(viewport("float-1"));
    multi.passes(2, frame(&mut app));

    assert_eq!(tabs(&app, "side"), ["S", "B"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_window_events_are_emitted() {
    let bus = Arc::new(TestBus::default());
    let heard: Arc<Mutex<Vec<String>>> = Arc::default();
    {
        let bus: &dyn Bus = &*bus;
        let h = heard.clone();
        bus.listen::<bus::WindowDetached>("test", ListenOpts::new(), move |info, _| {
            h.lock()
                .unwrap()
                .push(format!("detached {} {:?}", info.key, info.tabs));
            Ok(())
        })
        .unwrap();
        let h = heard.clone();
        bus.listen::<bus::WindowClosed>("test", ListenOpts::new(), move |info, _| {
            h.lock().unwrap().push(format!(
                "closed {} {:?} {:?}",
                info.key, info.policy, info.moved
            ));
            Ok(())
        })
        .unwrap();
        let h = heard.clone();
        bus.listen::<bus::LayoutChanged>("test", ListenOpts::new(), move |change, _| {
            h.lock()
                .unwrap()
                .push(format!("changed {:?}", change.change));
            Ok(())
        })
        .unwrap();
    }
    let mut multi = Multi::new();
    let mut app = app(bus);
    detach_b(&mut multi, &mut app);
    multi.request_close(viewport("float-1"));
    multi.passes(2, frame(&mut app));

    let heard = heard.lock().unwrap().clone();
    assert_eq!(
        heard,
        [
            "detached float-1 [\"B\"]",
            "changed Detached",
            "closed float-1 Some(ReturnToOrigin) [(\"B\", \"main\")]",
            "changed FloatClosed",
        ],
    );
}
