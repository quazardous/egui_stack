//! A whole tab group being dragged shows its ghost under the pointer, as a dragged tab does:
//! its tab bar with every tab, wherever the drag was started (the empty part of the tab bar,
//! Shift and a tab), and nothing of it once dropped.

use egui::{
    Event, Id, Modifiers, Pos2, Rect, Shape, Ui, UiBuilder, WidgetText,
    epaint::ClippedShape, pos2, vec2,
};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, NodeIndex, NodePath, SurfaceIndex, TabIndex, TabViewer};

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
