//! The default overlay, `OverlayType::Zones`: a tab dropped in the band along a leaf's edge splits
//! the leaf on that side, elsewhere in the leaf it joins its tabs, and outside the dock it opens
//! in a new window.

use egui::{Id, Pos2, Rect, Ui, UiBuilder, WidgetText, pos2, vec2};
use egui_kittest::Harness;
use egui_stack::{DockArea, DockState, NodeIndex, OverlayType, Style, SurfaceIndex, TabViewer};

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
}

/// `First` and `Second` on the left, `Third` on the right. The dock takes the left 600 points
/// of an 800 × 400 screen, so the rest is outside it.
fn harness() -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec!["First".to_owned(), "Second".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    let app = App {
        dock_state,
        viewer: Viewer {
            tab_rects: Vec::new(),
        },
    };
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            app.viewer.tab_rects.clear();
            let style = Style::from_egui(ui.style());
            assert_eq!(style.overlay.overlay_type, OverlayType::Zones);
            let dock = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0));
            ui.scope_builder(UiBuilder::new().max_rect(dock), |ui| {
                DockArea::new(&mut app.dock_state)
                    .style(style)
                    .show_inside(ui, &mut app.viewer);
            });
        },
        app,
    );
    harness.run();
    harness
}

fn third_leaf(harness: &Harness<'_, App>) -> Rect {
    harness.state().dock_state.main_surface()[NodeIndex(2)]
        .rect()
        .unwrap()
}

/// Drags `First` to where `target` says, holds it there, then releases it.
fn drop_first_at(harness: &mut Harness<'_, App>, target: impl Fn(&Harness<'_, App>) -> Pos2) {
    let start = harness
        .state()
        .viewer
        .tab_rects
        .iter()
        .find(|(tab, _)| tab == "First")
        .map(|(_, rect)| rect.center())
        .unwrap();
    let target = target(harness);
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
    harness.drop_at(target);
    harness.run_steps(5);
}

/// The leaves of the main surface, left to right and top to bottom.
fn leaves(app: &App) -> Vec<Vec<String>> {
    app.dock_state
        .main_surface()
        .iter()
        .filter_map(|node| node.tabs().map(<[String]>::to_vec))
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_middle_of_a_leaf_adds_the_tab_to_it() {
    let mut harness = harness();
    drop_first_at(&mut harness, |h| {
        let leaf = third_leaf(h);
        pos2(leaf.center().x, leaf.center().y + 30.0)
    });
    assert_eq!(
        leaves(harness.state()),
        [
            vec!["Second".to_owned()],
            vec!["Third".to_owned(), "First".to_owned()]
        ]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn an_edge_band_splits_the_leaf_on_that_side() {
    for (name, at, split_vertically) in [
        (
            "right",
            (|leaf: Rect| pos2(leaf.right() - 10.0, leaf.center().y)) as fn(Rect) -> Pos2,
            false,
        ),
        (
            "bottom",
            |leaf: Rect| pos2(leaf.center().x, leaf.bottom() - 10.0),
            true,
        ),
    ] {
        let mut harness = harness();
        let before = third_leaf(&harness);
        drop_first_at(&mut harness, |h| at(third_leaf(h)));

        let surface = harness.state().dock_state.main_surface();
        let third = surface
            .find_tab(&"Third".to_owned())
            .map(|(node, _)| surface[node].rect().unwrap())
            .unwrap();
        let first = surface
            .find_tab(&"First".to_owned())
            .map(|(node, _)| surface[node].rect().unwrap())
            .unwrap();
        assert_ne!(third, first, "{name}: First has a leaf of its own");
        assert!(
            before.contains_rect(first),
            "{name}: in Third's former place"
        );
        if split_vertically {
            assert!(
                first.top() > third.bottom() - 1.0,
                "{name}: First below Third"
            );
        } else {
            assert!(
                first.left() > third.right() - 1.0,
                "{name}: First right of Third"
            );
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn outside_the_dock_the_tab_opens_in_a_window() {
    let mut harness = harness();
    drop_first_at(&mut harness, |_| pos2(700.0, 200.0));
    let first = harness
        .state()
        .dock_state
        .find_tab(&"First".to_owned())
        .expect("First is still docked somewhere");
    assert_ne!(first.surface, SurfaceIndex::main(), "First is in a window");
    assert!(
        harness
            .state()
            .dock_state
            .get_window_state(first.surface)
            .is_some()
    );
    assert_eq!(
        leaves(harness.state()),
        [vec!["Second".to_owned()], vec!["Third".to_owned()]]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_window_takes_drops_on_its_zones() {
    let mut harness = harness();
    let third = harness
        .state()
        .dock_state
        .find_tab(&"Third".to_owned())
        .unwrap();
    let window = harness.state_mut().dock_state.detach_tab(
        third,
        Rect::from_min_size(pos2(250.0, 120.0), vec2(300.0, 220.0)),
    );
    harness.run();
    let leaf = |h: &Harness<'_, App>| {
        h.state().dock_state[window][NodeIndex::root()]
            .rect()
            .unwrap()
    };
    // In the band along the window leaf's left edge: a split inside the window.
    drop_first_at(&mut harness, |h| {
        let leaf = leaf(h);
        pos2(leaf.left() + 10.0, leaf.center().y)
    });
    let state = &harness.state().dock_state;
    let first = state.find_tab(&"First".to_owned()).unwrap();
    assert_eq!(first.surface, window, "First went into the window");
    let window_tabs: Vec<Vec<String>> = state[window]
        .iter()
        .filter_map(|node| node.tabs().map(<[String]>::to_vec))
        .collect();
    assert_eq!(
        window_tabs,
        [vec!["First".to_owned()], vec!["Third".to_owned()]]
    );
}
