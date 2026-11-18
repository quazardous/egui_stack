//! A drag whose release the application never sees ends as a cancel, as Escape does: the
//! layout stays as it was, and the ghost goes. On Wayland, a screenshot taken during a drag
//! grabs the pointer; the button is let go while the compositor holds it, so no release comes.
//! What the window sees instead: it loses the focus, or, back in it, a new press while egui
//! still believes the button held.

use egui::{
    Event, Id, PointerButton, Pos2, Rect, Shape, Ui, WidgetText, accesskit::Role,
    epaint::ClippedShape, vec2,
};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, NodeIndex, TabPath, TabViewer};

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

fn harness() -> Harness<'static, DockState<String>> {
    let mut dock_state = DockState::new(vec!["First".to_owned(), "Second".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, dock_state: &mut DockState<String>| {
            DockArea::new(dock_state).show_inside(ui, &mut Viewer);
        },
        dock_state,
    );
    harness.run();
    harness
}

/// Every tab and where it is, windows included.
fn layout(dock_state: &DockState<String>) -> Vec<(TabPath, String)> {
    dock_state
        .iter_all_tabs()
        .map(|(path, tab)| (path, tab.clone()))
        .collect()
}

/// Where the shapes drawing `text` are, in the last frame's output.
fn text_rects(shapes: &[ClippedShape], text: &str) -> Vec<Rect> {
    fn visit(shape: &Shape, text: &str, rects: &mut Vec<Rect>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| visit(s, text, rects)),
            Shape::Text(t) if t.galley.text() == text => rects.push(shape.visual_bounding_rect()),
            _ => {}
        }
    }
    let mut rects = Vec::new();
    for clipped in shapes {
        visit(&clipped.shape, text, &mut rects);
    }
    rects
}

/// Presses on `First` and drags it over the middle of the leaf holding `Third`, without
/// releasing; checks its ghost is there. Returns where the pointer is.
fn drag_first_over_third(harness: &mut Harness<'_, DockState<String>>) -> Pos2 {
    let start = harness
        .get_by_role_and_label(Role::Tab, "First")
        .rect()
        .center();
    let leaf = harness.state().main_surface()[NodeIndex(2)].rect().unwrap();
    let target = leaf.center();

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
        ghost_shown(harness, start),
        "while dragged, the ghost follows the pointer"
    );
    target
}

/// Whether `First`'s title is drawn away from its tab bar: its ghost.
fn ghost_shown(harness: &Harness<'_, DockState<String>>, start: Pos2) -> bool {
    text_rects(&harness.output().shapes, "First")
        .iter()
        .any(|rect| rect.top() > start.y + 40.0)
}

/// After the drag was lost: the pointer moves on (egui still believes the button held), then
/// the next release comes. Neither brings the drag back nor drops the tab.
fn after_the_loss(
    harness: &mut Harness<'_, DockState<String>>,
    before: &[(TabPath, String)],
    start: Pos2,
    target: Pos2,
) {
    for step in 1..=5 {
        harness.hover_at(target + vec2(4.0 * step as f32, 3.0 * step as f32));
        harness.step();
        assert!(
            !ghost_shown(harness, start),
            "pass {step} after the loss: no ghost"
        );
    }
    assert_eq!(layout(harness.state()), before, "nothing moved");
    harness.drop_at(target);
    harness.run_steps(5);
    assert_eq!(
        layout(harness.state()),
        before,
        "a later release drops nothing"
    );
    assert!(!ghost_shown(harness, start), "no ghost once released");

    // The next drag works as usual.
    let target = drag_first_over_third(harness);
    harness.drop_at(target);
    harness.run_steps(5);
    let tabs: Vec<_> = layout(harness.state())
        .into_iter()
        .map(|(_, tab)| tab)
        .collect();
    assert_eq!(tabs, ["Second", "Third", "First"], "the next drag drops");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_focus_lost_during_a_drag_cancels_it() {
    let mut harness = harness();
    let before = layout(harness.state());
    let start = harness
        .get_by_role_and_label(Role::Tab, "First")
        .rect()
        .center();
    let target = drag_first_over_third(&mut harness);

    harness.event(Event::WindowFocused(false));
    harness.step();
    harness.step();
    assert!(
        !ghost_shown(&harness, start),
        "the focus lost, the ghost goes"
    );
    harness.event(Event::WindowFocused(true));
    after_the_loss(&mut harness, &before, start, target);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_press_during_a_drag_cancels_it() {
    let mut harness = harness();
    let before = layout(harness.state());
    let start = harness
        .get_by_role_and_label(Role::Tab, "First")
        .rect()
        .center();
    let target = drag_first_over_third(&mut harness);

    // The release went elsewhere; back in the window, the user presses again.
    harness.event(Event::PointerButton {
        pos: target,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    harness.step();
    harness.step();
    assert!(
        !ghost_shown(&harness, start),
        "pressed again, the ghost goes"
    );
    assert_eq!(layout(harness.state()), before, "nothing moved");
    after_the_loss(&mut harness, &before, start, target);
}
