//! A whole tab group being dragged shows its ghost under the pointer, as a dragged tab does:
//! its tab bar with every tab, wherever the drag was started (the empty part of the tab bar,
//! Shift and a tab), and nothing of it once dropped. The ghost is painted through
//! `DockPainter::dragged_group`, the active tab marked, a large group clamped with a "+N".

use egui::{
    Event, Id, Modifiers, Pos2, Rect, Shape, Ui, UiBuilder, WidgetText, epaint::ClippedShape, pos2,
    vec2,
};
use egui_kittest::{Harness, kittest::Queryable as _};
use std::sync::Mutex;

use egui_stack::{
    DockArea, DockPainter, DockState, DraggedGroupPaint, NodeIndex, NodePath, PartState,
    StylePainter, SurfaceIndex, TabIndex, TabViewer,
};

struct Viewer;

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
}

const GROUP: [&str; 3] = ["Bravo", "Charlie", "Delta"];

/// `Alpha` on the left; `Bravo`, `Charlie`, `Delta` on the right, `Charlie` shown. The dock
/// takes the left 600 points of an 800 × 400 screen.
fn harness() -> Harness<'static, DockState<String>> {
    let mut dock_state = DockState::new(vec!["Alpha".to_owned()]);
    let [_, right] = dock_state.main_surface_mut().split_right(
        NodeIndex::root(),
        0.5,
        GROUP.iter().map(|s| (*s).to_owned()).collect(),
    );
    dock_state
        .leaf_mut(NodePath::new(SurfaceIndex::main(), right))
        .unwrap()
        .active = TabIndex(1);
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, dock_state: &mut DockState<String>| {
            let dock = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0));
            ui.scope_builder(UiBuilder::new().max_rect(dock), |ui| {
                DockArea::new(dock_state).show_inside(ui, &mut Viewer);
            });
        },
        dock_state,
    );
    harness.run();
    harness
}

/// Where the shapes drawing `text` are, in the last frame's output.
fn text_rects(shapes: &[ClippedShape], text: &str) -> Vec<Rect> {
    fn visit(shape: &Shape, clip: Rect, text: &str, rects: &mut Vec<Rect>) {
        match shape {
            Shape::Vec(shapes) => {
                for shape in shapes {
                    visit(shape, clip, text, rects);
                }
            }
            Shape::Text(text_shape) if text_shape.galley.text() == text => {
                rects.push(shape.visual_bounding_rect().intersect(clip));
            }
            _ => {}
        }
    }
    let mut rects = Vec::new();
    for clipped in shapes {
        visit(&clipped.shape, clipped.clip_rect, text, &mut rects);
    }
    rects
}

/// Whether every tab of the group is drawn near `pointer`, away from the tab bars at the top.
fn ghost_under(harness: &Harness<'_, DockState<String>>, pointer: Pos2) -> bool {
    let shapes = &harness.output().shapes;
    GROUP.iter().all(|title| {
        text_rects(shapes, title).iter().any(|rect| {
            rect.is_positive() && rect.top() > 60.0 && rect.center().distance(pointer) < 250.0
        })
    })
}

/// Drags from `start` to `target` with `modifiers` held, checks the ghost there, then drops.
fn drag_and_check(harness: &mut Harness<'_, DockState<String>>, start: Pos2, modifiers: Modifiers) {
    let target = pos2(150.0, 250.0);
    harness.event(Event::ModifiersChanged(modifiers));
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=10 {
        harness.hover_at(start.lerp(target, step as f32 / 10.0));
        harness.step();
    }
    for _ in 0..5 {
        harness.hover_at(target);
        harness.step();
    }
    assert!(
        ghost_under(harness, target),
        "while the group is dragged, its ghost (every tab's title) follows the pointer"
    );

    harness.drop_at(target);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run_steps(5);
    assert!(
        !ghost_under(harness, target),
        "once dropped, no ghost is left under the pointer"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_group_dragged_by_its_tab_bar_shows_its_ghost() {
    let mut harness = harness();
    let surface = harness.state().main_surface();
    let (node, _) = surface.find_tab(&"Bravo".to_owned()).unwrap();
    let leaf = surface[node].rect().unwrap();
    let tail = pos2(leaf.right() - 30.0, leaf.top() + 10.0);
    drag_and_check(&mut harness, tail, Modifiers::NONE);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_group_dragged_by_shift_and_a_tab_shows_its_ghost() {
    let mut harness = harness();
    let tab = harness.get_by_label("Bravo").rect().center();
    drag_and_check(&mut harness, tab, Modifiers::SHIFT);
}

/// What `DockPainter::dragged_group` was given: the titles shown, which one is selected, the
/// "+N" count and the ghost's rect.
#[derive(Clone, Debug, Default)]
struct Seen {
    titles: Vec<String>,
    selected: Vec<String>,
    hidden: Option<usize>,
    count: usize,
    rect: Option<Rect>,
}

/// A painter that records the group ghost it is asked to paint, then paints it as by default.
#[derive(Default)]
struct Recorder(Mutex<Vec<Seen>>);

impl DockPainter for Recorder {
    fn dragged_group(&self, painter: &egui::Painter, group: &DraggedGroupPaint<'_>) {
        self.0.lock().unwrap().push(Seen {
            titles: group
                .tabs
                .iter()
                .map(|t| t.title.text().to_owned())
                .collect(),
            selected: group
                .tabs
                .iter()
                .filter(|t| t.state.contains(PartState::SELECTED))
                .map(|t| t.title.text().to_owned())
                .collect(),
            hidden: group.more.as_ref().map(|more| more.hidden),
            count: group.count,
            rect: Some(group.rect),
        });
        StylePainter.dragged_group(painter, group);
    }
}

/// `Alpha` on the left, `tabs` on the right with `active` shown, painted by `recorder`; drags
/// the right group by its tab bar to the left leaf and returns what the painter saw last while
/// it was held there, and the pointer.
fn record(tabs: Vec<String>, active: usize, recorder: &'static Recorder) -> (Seen, Pos2) {
    let mut dock_state = DockState::new(vec!["Alpha".to_owned()]);
    let [_, right] = dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, tabs);
    dock_state
        .leaf_mut(NodePath::new(SurfaceIndex::main(), right))
        .unwrap()
        .active = TabIndex(active);
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        move |ui, dock_state: &mut DockState<String>| {
            let dock = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0));
            ui.scope_builder(UiBuilder::new().max_rect(dock), |ui| {
                DockArea::new(dock_state)
                    .painter(recorder)
                    .show_inside(ui, &mut Viewer);
            });
        },
        dock_state,
    );
    harness.run();
    let leaf = harness.state().main_surface()[right].rect().unwrap();
    let start = pos2(leaf.right() - 20.0, leaf.top() + 10.0);
    let target = pos2(150.0, 250.0);
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=10 {
        harness.hover_at(start.lerp(target, step as f32 / 10.0));
        harness.step();
    }
    harness.hover_at(target);
    harness.step();
    let seen = recorder
        .0
        .lock()
        .unwrap()
        .last()
        .cloned()
        .expect("a ghost painted");
    harness.drop_at(target);
    harness.run_steps(3);
    recorder.0.lock().unwrap().clear();
    harness.hover_at(target);
    harness.run_steps(3);
    assert!(
        recorder.0.lock().unwrap().is_empty(),
        "no ghost painted once dropped"
    );
    (seen, target)
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_painter_paints_the_ghost_with_every_tab_and_the_active_one_marked() {
    static RECORDER: std::sync::LazyLock<Recorder> = std::sync::LazyLock::new(Recorder::default);
    let tabs = GROUP.iter().map(|s| (*s).to_owned()).collect();
    let (seen, pointer) = record(tabs, 1, &RECORDER);
    assert_eq!(seen.titles, GROUP);
    assert_eq!(seen.selected, ["Charlie"]);
    assert_eq!(seen.hidden, None);
    assert_eq!(seen.count, 3);
    assert!(
        seen.rect.unwrap().expand(1.0).contains(pointer),
        "the ghost is under the pointer: {seen:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_large_group_keeps_a_sensible_ghost_and_sums_up_the_rest() {
    static RECORDER: std::sync::LazyLock<Recorder> = std::sync::LazyLock::new(Recorder::default);
    let tabs: Vec<String> = (0..30).map(|i| format!("Document {i}")).collect();
    let (seen, _) = record(tabs, 20, &RECORDER);
    let rect = seen.rect.unwrap();
    assert!(
        rect.width() <= 320.0 && rect.height() <= 200.0,
        "clamped: {rect:?}"
    );
    assert_eq!(seen.count, 30);
    assert!(seen.titles.len() < 30, "not every tab fits: {seen:?}");
    assert_eq!(seen.hidden, Some(30 - seen.titles.len()), "the rest as +N");
    assert_eq!(
        seen.selected,
        ["Document 20"],
        "the active tab is always shown"
    );
}
