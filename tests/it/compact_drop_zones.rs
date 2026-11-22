//! In a compact leaf, a tab dropped right under the tab bar joins the leaf's tabs, as over the
//! tab bar ([`OverlayFeel::zone_tab_bar_stack_band`]), and the top edge band, measured on the
//! leaf's body below the tab bar ([`OverlayFeel::zone_edges_below_tab_bar`]), still splits it.
//! The former geometry, bands measured on the whole leaf, stays reachable through the style.
//!
//! [`OverlayFeel::zone_tab_bar_stack_band`]: egui_stack::OverlayFeel::zone_tab_bar_stack_band
//! [`OverlayFeel::zone_edges_below_tab_bar`]: egui_stack::OverlayFeel::zone_edges_below_tab_bar

use egui::{Event, Id, Modifiers, PointerButton, Pos2, Rect, Ui, WidgetText, pos2, vec2};
use egui_kittest::Harness;
use egui_stack::{DockArea, DockState, NodeIndex, OverlayFeel, OverlayType, Style, TabViewer};

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
    feel: fn(&mut OverlayFeel),
}

const TAB_BAR: f32 = 28.0;
const LEAF_HEIGHT: f32 = 96.0;
/// The harness's panel margin, around the dock.
const MARGIN: f32 = 8.0;

/// `A` on the left half of an 800-point-wide screen, `B` on the right: two compact leaves with 28-point
/// tab bars; `feel` tunes the overlay. 60 frames a second.
fn harness(feel: fn(&mut OverlayFeel)) -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec!["A".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["B".to_owned()]);
    let app = App {
        dock_state,
        viewer: Viewer {
            tab_rects: Vec::new(),
        },
        feel,
    };
    let mut harness = Harness::builder()
        .with_size([800.0, LEAF_HEIGHT + 2.0 * MARGIN])
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            |ui, app: &mut App| {
                app.viewer.tab_rects.clear();
                let mut style = Style::from_egui(ui.style());
                assert_eq!(style.overlay.overlay_type, OverlayType::Zones);
                style.tab_bar.height = TAB_BAR;
                (app.feel)(&mut style.overlay.feel);
                DockArea::new(&mut app.dock_state)
                    .style(style)
                    .show_inside(ui, &mut app.viewer);
            },
            app,
        );
    harness.run();
    let a = leaves(harness.state())[0].1;
    assert_eq!(a.height(), LEAF_HEIGHT, "the leaf is compact");
    harness
}

/// The leaves of the main surface with their rects, in tree order.
fn leaves(app: &App) -> Vec<(Vec<String>, Rect)> {
    app.dock_state
        .main_surface()
        .iter()
        .filter_map(|node| Some((node.tabs()?.to_vec(), node.rect()?)))
        .collect()
}

/// Drags `B` into `A`'s leaf, `below` points under the top of its leaf, and drops it there.
fn drop_b_at(harness: &mut Harness<'_, App>, below: f32) {
    let a = leaves(harness.state())[0].1;
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
    for step in 1..=5 {
        harness.hover_at(start + vec2(0.0, 10.0 * step as f32));
        harness.step();
    }
    let target: Pos2 = pos2(a.center().x, a.top() + below);
    // Half a second over the target: longer than the leaf preference
    // (`OverlayFeel::max_preference_time`) keeps `B`'s own leaf.
    for _ in 0..30 {
        harness.hover_at(target);
        harness.step();
    }
    harness.input_mut().events.push(Event::PointerButton {
        pos: target,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(5);
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

fn default_feel(_: &mut OverlayFeel) {}

/// The geometry before the stack band: edge bands on the whole leaf, tab bar included.
fn former_feel(feel: &mut OverlayFeel) {
    feel.zone_edges_below_tab_bar = false;
    feel.zone_tab_bar_stack_band = 0.0;
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn dropped_just_under_the_tab_bar_it_stacks() {
    // Half a point under the tab bar: the former geometry split the leaf there (see below).
    let mut harness = harness(default_feel);
    drop_b_at(&mut harness, TAB_BAR + 0.5);
    assert!(stacked(harness.state()), "{:?}", leaves(harness.state()));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn dropped_in_the_top_edge_band_it_splits() {
    // The default stack band is 8 points; the top band, 30 % of the 60 points of body below
    // it, reaches 18 points further down.
    let mut harness = harness(default_feel);
    drop_b_at(&mut harness, TAB_BAR + 8.0 + 9.0);
    assert!(
        split_above(harness.state()),
        "{:?}",
        leaves(harness.state())
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn below_the_top_edge_band_it_stacks() {
    let mut harness = harness(default_feel);
    drop_b_at(&mut harness, LEAF_HEIGHT / 2.0 + 14.0);
    assert!(stacked(harness.state()), "{:?}", leaves(harness.state()));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_former_geometry_stays_reachable() {
    // 30 % of the whole 96-point leaf: the top band ends 28.8 points down, under a 28-point tab
    // bar.
    let mut sliver = harness(former_feel);
    drop_b_at(&mut sliver, TAB_BAR + 0.5);
    assert!(split_above(sliver.state()), "{:?}", leaves(sliver.state()));
    let mut lower = harness(former_feel);
    drop_b_at(&mut lower, TAB_BAR + 8.0 + 9.0);
    assert!(stacked(lower.state()), "{:?}", leaves(lower.state()));
}
