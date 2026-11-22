//! A tab is dropped where the pointer is released, not where it was a frame earlier: released on
//! a leaf's tab bar it joins the leaf's tabs, released in an edge band it splits the leaf, even
//! when the pointer moved there in the release frame or the hovered leaf is still preferred
//! ([`OverlayFeel::max_preference_time`](egui_stack::OverlayFeel::max_preference_time)).

use egui::{Event, Id, Modifiers, PointerButton, Pos2, Rect, Ui, WidgetText, pos2, vec2};
use egui_kittest::Harness;
use egui_stack::{DockArea, DockState, NodeIndex, OverlayType, Style, TabViewer};

struct Viewer {
    tab_rects: Vec<(String, Rect)>,
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

    fn on_tab_button(&mut self, tab: &mut String, response: &egui::Response) {
        self.tab_rects.push((tab.clone(), response.rect));
    }
}

struct App {
    dock_state: DockState<String>,
    viewer: Viewer,
    preference_time: f32,
}

/// `A` on the left half of an 800 × 400 screen, `B` on the right; 24-point tab bars, the default
/// edge bands (30 %), the leaf preference held for `preference_time` seconds, 60 frames a second.
fn harness(preference_time: f32) -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec!["A".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["B".to_owned()]);
    let app = App {
        dock_state,
        viewer: Viewer {
            tab_rects: Vec::new(),
        },
        preference_time,
    };
    let mut harness = Harness::builder()
        .with_size([800.0, 400.0])
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            |ui, app: &mut App| {
                app.viewer.tab_rects.clear();
                let mut style = Style::from_egui(ui.style());
                assert_eq!(style.overlay.overlay_type, OverlayType::Zones);
                assert_eq!(style.tab_bar.height, 24.0);
                assert_eq!(style.overlay.feel.zone_edge_coverage, 0.3);
                style.overlay.feel.max_preference_time = app.preference_time;
                DockArea::new(&mut app.dock_state)
                    .style(style)
                    .show_inside(ui, &mut app.viewer);
            },
            app,
        );
    harness.run();
    harness
}

/// Drags `B` over `path` (each point held `frames` frames), then moves to `release` and releases
/// there in the same frame.
fn drag_b(harness: &mut Harness<'_, App>, path: &[(Pos2, usize)], release: Pos2) {
    let start = harness
        .state()
        .viewer
        .tab_rects
        .iter()
        .find(|(tab, _)| tab == "B")
        .map(|(_, rect)| rect.center())
        .unwrap();
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=3 {
        harness.hover_at(start + vec2(0.0, 10.0 * step as f32));
        harness.step();
    }
    for &(at, frames) in path {
        for _ in 0..frames {
            harness.hover_at(at);
            harness.step();
        }
    }
    // One frame: the pointer moves and the button is released, as a fast mouse gives. (The
    // harness's `hover_at` and `drop_at` would give each event a frame of its own.)
    let events = &mut harness.input_mut().events;
    events.push(Event::PointerMoved(release));
    events.push(Event::PointerButton {
        pos: release,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(5);
}

/// The leaves of the main surface with their rects, in tree order.
fn leaves(app: &App) -> Vec<(Vec<String>, Rect)> {
    app.dock_state
        .main_surface()
        .iter()
        .filter_map(|node| Some((node.tabs()?.to_vec(), node.rect()?)))
        .collect()
}

fn stacked(app: &App) -> bool {
    let leaves = leaves(app);
    leaves.len() == 1 && leaves[0].0 == ["A", "B"]
}

fn split_above(app: &App) -> bool {
    let leaves = leaves(app);
    let rect = |tab: &str| {
        leaves
            .iter()
            .find(|(tabs, _)| tabs == &[tab.to_owned()])
            .map(|(_, rect)| *rect)
    };
    leaves.len() == 2
        && rect("A")
            .zip(rect("B"))
            .is_some_and(|(a, b)| b.bottom() <= a.top() + 1.0 && (b.left() - a.left()).abs() < 1.0)
}

const EDGE_BAND: Pos2 = pos2(200.0, 44.0);
const TAB_BAR: Pos2 = pos2(200.0, 12.0);
const CENTRE: Pos2 = pos2(200.0, 200.0);

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn released_on_the_tab_bar_right_after_an_edge_band_it_stacks() {
    let mut harness = harness(0.0);
    drag_b(&mut harness, &[(CENTRE, 3), (EDGE_BAND, 3)], TAB_BAR);
    assert!(stacked(harness.state()), "{:?}", leaves(harness.state()));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn released_in_an_edge_band_right_after_the_centre_it_splits() {
    let mut harness = harness(0.0);
    drag_b(&mut harness, &[(CENTRE, 3)], EDGE_BAND);
    assert!(
        split_above(harness.state()),
        "{:?}",
        leaves(harness.state())
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn released_on_the_tab_bar_while_the_leaf_is_preferred_it_stacks() {
    // The default preference time: at 60 frames a second, the body hover is kept for 18 frames
    // after the pointer leaves it for the tab bar.
    let mut harness = harness(0.3);
    drag_b(&mut harness, &[(EDGE_BAND, 25), (TAB_BAR, 1)], TAB_BAR);
    assert!(stacked(harness.state()), "{:?}", leaves(harness.state()));
}
