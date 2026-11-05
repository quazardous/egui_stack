//! The dragged tab's ghost follows the pointer while the drag lasts, and nothing of it stays once
//! the tab is dropped, even in the band along the right edge of a leaf at the screen's right edge.

use egui::{Id, Rect, Shape, Ui, WidgetText, accesskit::Role, epaint::ClippedShape, pos2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, NodeIndex, TabViewer};

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

const SIZE: [f32; 2] = [600.0, 300.0];

/// `First` and `Second` on the left, `Third` on the right, the dock filling the screen.
fn harness() -> Harness<'static, DockState<String>> {
    let mut dock_state = DockState::new(vec!["First".to_owned(), "Second".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    let mut harness = Harness::builder().with_size(SIZE).build_ui_state(
        |ui, dock_state: &mut DockState<String>| {
            DockArea::new(dock_state).show_inside(ui, &mut Viewer);
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

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn no_ghost_is_left_after_a_drop_at_the_screen_edge() {
    let mut harness = harness();
    let start = harness
        .get_by_role_and_label(Role::Tab, "Second")
        .rect()
        .center();
    let third = harness.state().main_surface()[NodeIndex(2)].rect().unwrap();
    assert!(
        third.right() >= SIZE[0] - 10.0,
        "the right leaf reaches the screen's right edge"
    );
    let target = pos2(third.right() - 2.0, third.center().y);

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
    // The test sees the ghost: while dragged, the title is drawn away from the tab bar.
    let dragging = text_rects(&harness.output().shapes, "Second");
    assert!(
        dragging.iter().any(|rect| rect.top() > start.y + 50.0),
        "while dragged, the ghost follows the pointer: {dragging:?}"
    );

    harness.drop_at(target);
    harness.run_steps(10);

    let leaves: Vec<Vec<String>> = harness
        .state()
        .main_surface()
        .iter()
        .filter_map(|node| node.tabs().map(<[String]>::to_vec))
        .collect();
    assert_eq!(
        leaves,
        [
            vec!["First".to_owned()],
            vec!["Third".to_owned()],
            vec!["Second".to_owned()]
        ],
        "the drop split the right leaf, the tab on the right"
    );
    let (node, _) = harness
        .state()
        .main_surface()
        .find_tab(&"Second".to_owned())
        .unwrap();
    let new_leaf = harness.state().main_surface()[node].rect().unwrap();

    for _ in 0..3 {
        harness.hover_at(target);
        harness.step();
        let dropped = text_rects(&harness.output().shapes, "Second");
        assert_eq!(
            dropped.len(),
            1,
            "only the tab's title is drawn, no ghost: {dropped:?}"
        );
        assert!(
            new_leaf.contains_rect(dropped[0]),
            "the title is in the new leaf {new_leaf:?}: {dropped:?}"
        );
    }
}
