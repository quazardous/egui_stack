//! A custom `DockPainter` measures and paints the dock's parts in place of the style.

use std::sync::Mutex;

use egui::{Color32, Id, Painter, Rect, Ui, WidgetText, accesskit::Role, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{
    DockArea, DockPainter, DockState, DropPaint, NodeIndex, PartState, SeparatorAxis,
    SeparatorPaint, TabBarPaint, TabDestination, TabMeasure, TabPaint, TabViewer,
    tab_viewer::DropVerdict,
};

const TAB_WIDTH: f32 = 30.0;
const INK: Color32 = Color32::from_rgb(1, 2, 3);

/// Every tab 30 points wide, whatever its title, and a record of what was painted.
#[derive(Default)]
struct Recorder {
    bars: Mutex<Vec<Rect>>,
    tabs: Mutex<Vec<(String, Rect, PartState, bool)>>,
    separators: Mutex<Vec<(SeparatorAxis, PartState)>>,
    drops: Mutex<Vec<(bool, Option<String>)>>,
}

impl DockPainter for Recorder {
    fn tab_width(&self, _tab: &TabMeasure<'_>) -> f32 {
        TAB_WIDTH
    }

    fn tab_bar(&self, painter: &Painter, bar: &TabBarPaint<'_>) {
        self.bars.lock().unwrap().push(bar.rect);
        painter.rect_filled(bar.rect, 0.0, INK);
    }

    fn tab(&self, painter: &Painter, tab: &TabPaint<'_>) {
        self.tabs.lock().unwrap().push((
            tab.title.text().to_owned(),
            tab.rect,
            tab.state,
            tab.close.is_some(),
        ));
        painter.rect_filled(tab.rect, 0.0, INK);
    }

    fn separator(&self, painter: &Painter, separator: &SeparatorPaint<'_>) {
        self.separators
            .lock()
            .unwrap()
            .push((separator.axis, separator.state));
        painter.rect_filled(separator.rect, 0.0, INK);
    }

    fn drop_preview(&self, _painter: &Painter, drop: &DropPaint<'_>) {
        self.drops.lock().unwrap().push((
            drop.refusal.is_some(),
            drop.refusal
                .and_then(|r| r.reason.as_ref())
                .map(|g| g.text().to_owned()),
        ));
    }
}

struct Viewer {
    refuse: bool,
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
        ui.label(format!("Body of {tab}"));
    }

    fn drop_allowed(&self, _tab: &String, _destination: &TabDestination) -> DropVerdict {
        if self.refuse {
            DropVerdict::Refuse(Some("Not here".into()))
        } else {
            DropVerdict::Allow
        }
    }
}

struct App {
    dock_state: DockState<String>,
    painter: Recorder,
    viewer: Viewer,
}

fn harness(refuse: bool) -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec!["A long first title".to_owned(), "Second".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    let app = App {
        dock_state,
        painter: Recorder::default(),
        viewer: Viewer { refuse },
    };
    Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            app.painter = Recorder::default();
            DockArea::new(&mut app.dock_state)
                .painter(&app.painter)
                .show_close_buttons(false)
                .show_inside(ui, &mut app.viewer);
        },
        app,
    )
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_painter_measures_and_paints_the_parts() {
    let mut harness = harness(false);
    harness.run();

    // The measure wins over the title's width: a tab can be narrower than its title.
    let tab = harness.get_by_role_and_label(Role::Tab, "A long first title");
    assert!((tab.rect().width() - TAB_WIDTH).abs() < 0.01);

    let app = harness.state();
    assert_eq!(app.painter.bars.lock().unwrap().len(), 2);
    let tabs = app.painter.tabs.lock().unwrap();
    let titles: Vec<&str> = tabs.iter().map(|(t, ..)| t.as_str()).collect();
    assert_eq!(titles, ["A long first title", "Second", "Third"]);
    let state_of = |title: &str| tabs.iter().find(|(t, ..)| t == title).unwrap().2;
    assert!(state_of("A long first title").contains(PartState::SELECTED));
    assert!(!state_of("Second").contains(PartState::SELECTED));
    assert!(state_of("Third").contains(PartState::SELECTED));
    assert!(tabs.iter().all(|(.., close)| !close));
    assert_eq!(
        *app.painter.separators.lock().unwrap(),
        [(SeparatorAxis::Vertical, PartState::NONE)]
    );

    // Nothing of the style's tab look is left: every tab is in the painter's ink.
    let filled_with_ink = |rect: Rect| {
        harness.output().shapes.iter().any(|clipped| {
            matches!(&clipped.shape, egui::Shape::Rect(r) if r.rect == rect && r.fill == INK)
        })
    };
    assert!(tabs.iter().all(|(_, rect, ..)| filled_with_ink(*rect)));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_painter_states_follow_the_pointer() {
    let mut harness = harness(false);
    harness.run();
    let second = harness.get_by_role_and_label(Role::Tab, "Second").rect();
    harness.hover_at(second.center());
    harness.run();
    let tabs = harness.state().painter.tabs.lock().unwrap();
    let second = tabs.iter().find(|(t, ..)| t == "Second").unwrap();
    assert!(second.2.contains(PartState::HOVERED));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_painter_shows_a_refused_drop() {
    let mut harness = harness(true);
    harness.run();
    let start = harness
        .get_by_role_and_label(Role::Tab, "A long first title")
        .rect()
        .center();
    let target = harness.state().dock_state.main_surface()[NodeIndex::root().right()]
        .rect()
        .unwrap()
        .center()
        + vec2(0.0, 30.0);
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=5 {
        harness.hover_at(start + vec2(0.0, 10.0 * step as f32));
        harness.step();
    }
    for _ in 0..5 {
        harness.hover_at(target);
        harness.step();
    }
    let drops = harness.state().painter.drops.lock().unwrap();
    assert_eq!(drops.last(), Some(&(true, Some("Not here".to_owned()))));
}
