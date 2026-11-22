//! A floating window keeps its decorations on its frame when it is shrunk past its content's
//! least size, and a folded window stays tall enough for its folded band.

use egui::{Id, Pos2, Rect, Ui, WidgetText, vec2};
use egui_kittest::{Harness, kittest::Queryable};
use egui_stack::{DockArea, DockState, NodeIndex, SurfaceIndex, TabViewer};

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
        // A line that does not wrap: the body's least width.
        ui.add(egui::Label::new(format!("The body of {tab}, on one long line")).extend());
    }

    fn scroll_bars(&self, _tab: &String) -> [bool; 2] {
        // No scroll area: the body's content sets the window's least width.
        [false, false]
    }
}

fn harness(tabs: &[&str]) -> (Harness<'static, DockState<String>>, SurfaceIndex) {
    let mut dock_state = DockState::new(vec!["Main".to_owned()]);
    let window = dock_state.add_window(tabs.iter().map(|tab| (*tab).to_owned()).collect());
    dock_state
        .get_window_state_mut(window)
        .unwrap()
        .set_position(Pos2::new(200.0, 100.0))
        .set_size(vec2(400.0, 200.0));
    (harness_of(dock_state), window)
}

/// Shows `dock_state` in a new harness.
fn harness_of(dock_state: DockState<String>) -> Harness<'static, DockState<String>> {
    let mut harness = Harness::builder().with_size([800.0, 500.0]).build_ui_state(
        |ui, dock_state: &mut DockState<String>| {
            DockArea::new(dock_state)
                .show_close_buttons(true)
                .show_leaf_close_all_buttons(true)
                .show_leaf_collapse_buttons(true)
                .show_inside(ui, &mut Viewer);
        },
        dock_state,
    );
    harness.run_steps(3);
    harness
}

fn window_rect(harness: &Harness<'_, DockState<String>>, window: SurfaceIndex) -> Rect {
    harness.state().get_window_state(window).unwrap().rect()
}

/// The rect of the button `label` of the floating window: the one below the main surface's bar.
fn button_rect(harness: &Harness<'_, DockState<String>>, label: &str) -> Rect {
    harness
        .query_all_by_label(label)
        .map(|node| node.rect())
        .find(|rect| rect.top() > 50.0)
        .unwrap_or_else(|| panic!("the window has a {label} button"))
}

/// Drags from `start` by `delta`, a step at a time, and releases.
fn drag(harness: &mut Harness<'_, DockState<String>>, start: Pos2, delta: egui::Vec2) {
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=10 {
        harness.hover_at(start + delta * (step as f32 / 10.0));
        harness.step();
    }
    harness.drop_at(start + delta);
    harness.run_steps(3);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn shrunk_past_its_least_width_a_window_keeps_its_buttons_at_its_edge() {
    let (mut harness, window) = self::harness(&["First tab", "Second tab", "Third tab"]);
    let rect = window_rect(&harness, window);
    let close = button_rect(&harness, "Close all tabs");
    let gap = rect.right() - close.right();

    drag(
        &mut harness,
        Pos2::new(rect.right() - 1.0, rect.center().y),
        vec2(-380.0, 0.0),
    );

    let shrunk = window_rect(&harness, window);
    let close = button_rect(&harness, "Close all tabs");
    assert!(shrunk.width() < rect.width(), "{shrunk:?}");
    assert!(shrunk.contains_rect(close), "{close:?} in {shrunk:?}");
    assert!(
        (shrunk.right() - close.right() - gap).abs() <= 1.0,
        "the close button is {} from the edge, was {gap}",
        shrunk.right() - close.right()
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn shrunk_past_its_least_size_a_split_window_keeps_every_bar_inside() {
    let (mut harness, window) = harness(&["First tab", "Second tab"]);
    harness.state_mut()[window].split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    harness.state_mut()[window].split_below(NodeIndex::root(), 0.5, vec!["Fourth".to_owned()]);
    harness.run_steps(3);
    let rect = window_rect(&harness, window);

    // Shrink from the bottom-right corner, far past what the content needs.
    drag(
        &mut harness,
        Pos2::new(rect.right() - 1.0, rect.bottom() - 1.0),
        vec2(-380.0, -190.0),
    );

    let shrunk = window_rect(&harness, window);
    assert!(
        shrunk.width() < rect.width() && shrunk.height() < rect.height(),
        "{shrunk:?}"
    );
    let buttons: Vec<Rect> = harness
        .query_all_by_label("Close all tabs")
        .map(|node| node.rect())
        .filter(|rect| rect.top() > 50.0)
        .collect();
    assert_eq!(buttons.len(), 3, "{buttons:?}");
    for button in buttons {
        assert!(shrunk.contains_rect(button), "{button:?} in {shrunk:?}");
    }
    // Each leaf is laid out within the frame, and the frame hugs its content.
    let content = harness.state()[window][NodeIndex::root()].rect().unwrap();
    assert!(shrunk.contains_rect(content), "{content:?} in {shrunk:?}");
    assert!(
        shrunk.right() - content.right() < 10.0,
        "{content:?} in {shrunk:?}"
    );
    assert!(
        shrunk.bottom() - content.bottom() < 10.0,
        "{content:?} in {shrunk:?}"
    );
}

/// Clicks the window's `label` button: the one below the main surface's bar.
fn click_window_button(harness: &mut Harness<'_, DockState<String>>, label: &str) {
    let rect = button_rect(harness, label);
    harness.hover_at(rect.center());
    harness.step();
    harness.drag_at(rect.center());
    harness.step();
    harness.drop_at(rect.center());
    harness.run_steps(4);
}

fn assert_bands_inside(harness: &Harness<'_, DockState<String>>, window: SurfaceIndex) {
    let rect = window_rect(harness, window);
    for label in ["Close all tabs", "Collapse"] {
        let buttons: Vec<Rect> = harness
            .query_all_by_label(label)
            .map(|node| node.rect())
            .filter(|rect| rect.top() > 50.0)
            .collect();
        assert!(!buttons.is_empty(), "the window has a {label} button");
        for button in buttons {
            assert!(rect.contains_rect(button), "{label} {button:?} in {rect:?}");
        }
    }
    for tab in harness.query_all_by_role(egui::accesskit::Role::Tab) {
        let tab = tab.rect();
        if tab.top() > 50.0 {
            assert!(rect.contains_rect(tab), "tab {tab:?} in {rect:?}");
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_folded_window_is_tall_enough_for_its_band() {
    let (mut harness, window) = harness(&["First tab", "Second tab"]);
    let rect = window_rect(&harness, window);

    click_window_button(&mut harness, "Collapse");
    let folded = window_rect(&harness, window);
    assert!(folded.height() < rect.height(), "{folded:?}");
    assert_bands_inside(&harness, window);

    click_window_button(&mut harness, "Collapse");
    assert!(window_rect(&harness, window).height() > folded.height());
}

/// Clicks the `index`th `label` button of the window, counted from the top.
fn click_nth_window_button(
    harness: &mut Harness<'_, DockState<String>>,
    label: &str,
    index: usize,
) {
    let mut rects: Vec<Rect> = harness
        .query_all_by_label(label)
        .map(|node| node.rect())
        .filter(|rect| rect.top() > 50.0)
        .collect();
    rects.sort_by(|a, b| a.top().total_cmp(&b.top()));
    let rect = rects[index];
    harness.hover_at(rect.center());
    harness.step();
    harness.drag_at(rect.center());
    harness.step();
    harness.drop_at(rect.center());
    harness.run_steps(4);
}

/// The heights of the window's leaves, from the top.
fn leaf_heights(harness: &Harness<'_, DockState<String>>, window: SurfaceIndex) -> Vec<f32> {
    let tree = &harness.state()[window];
    let mut rects: Vec<Rect> = tree
        .iter()
        .filter(|node| node.is_leaf())
        .filter_map(|node| node.rect())
        .collect();
    rects.sort_by(|a, b| a.top().total_cmp(&b.top()));
    rects.iter().map(Rect::height).collect()
}

fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= 1.0,
        "{what}: {actual}, expected {expected}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn an_unfolded_window_gets_back_its_height() {
    let (mut harness, window) = harness(&["First tab", "Second tab"]);
    let rect = window_rect(&harness, window);

    click_window_button(&mut harness, "Collapse");
    assert!(window_rect(&harness, window).height() < rect.height());

    click_window_button(&mut harness, "Collapse");
    harness.run_steps(3);
    let unfolded = window_rect(&harness, window);
    assert_close(unfolded.height(), rect.height(), "the window's height");
    assert_bands_inside(&harness, window);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_split_window_with_every_leaf_folded_unfolds_to_its_former_heights() {
    let (mut harness, window) = harness(&["First tab", "Second tab"]);
    harness.state_mut()[window].split_below(NodeIndex::root(), 0.3, vec!["Third".to_owned()]);
    harness.run_steps(3);
    let rect = window_rect(&harness, window);
    let heights = leaf_heights(&harness, window);
    assert_eq!(heights.len(), 2);

    // Fold the bottom leaf, then the top one: the window collapses to its two bands.
    click_nth_window_button(&mut harness, "Collapse", 1);
    click_nth_window_button(&mut harness, "Collapse", 0);
    harness.run_steps(3);
    let folded = window_rect(&harness, window);
    assert!(folded.height() < rect.height() / 2.0, "{folded:?}");
    assert_bands_inside(&harness, window);
    let folded_heights = leaf_heights(&harness, window);
    assert_close(folded_heights[0], folded_heights[1], "the folded bands");

    // Unfolding one leaf gives the window back its height, the open leaf taking the rest.
    click_nth_window_button(&mut harness, "Collapse", 0);
    harness.run_steps(3);
    let one_open = window_rect(&harness, window);
    assert_close(one_open.height(), rect.height(), "the window's height");
    assert_bands_inside(&harness, window);
    let one_open_heights = leaf_heights(&harness, window);
    assert!(
        one_open_heights[0] > one_open_heights[1],
        "{one_open_heights:?}"
    );

    // Unfolding the other gives back the heights from before.
    click_nth_window_button(&mut harness, "Collapse", 1);
    harness.run_steps(3);
    assert_close(
        window_rect(&harness, window).height(),
        rect.height(),
        "the window's height",
    );
    let restored = leaf_heights(&harness, window);
    for (restored, before) in restored.iter().zip(&heights) {
        assert_close(*restored, *before, "a leaf's height");
    }
    assert_bands_inside(&harness, window);
}

#[cfg(feature = "serde")]
#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_window_saved_folded_unfolds_to_its_height_once_loaded() {
    let (mut harness, window) = harness(&["First tab", "Second tab"]);
    let rect = window_rect(&harness, window);
    click_window_button(&mut harness, "Collapse");
    assert!(window_rect(&harness, window).height() < rect.height());

    let saved = serde_json::to_string(harness.state()).unwrap();
    let mut loaded: DockState<String> = serde_json::from_str(&saved).unwrap();
    // A new egui context places windows anew: put it back where it was.
    loaded
        .get_window_state_mut(window)
        .unwrap()
        .set_position(rect.min);
    let mut harness = harness_of(loaded);
    assert!(window_rect(&harness, window).height() < rect.height());

    click_window_button(&mut harness, "Collapse");
    harness.run_steps(3);
    assert_close(
        window_rect(&harness, window).height(),
        rect.height(),
        "the window's height",
    );
}
