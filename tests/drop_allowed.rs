//! `TabViewer::drop_allowed` can refuse a destination: the drop then leaves the layout unchanged.

use egui::{Id, Rect, Ui, WidgetText, vec2};
use egui_kittest::Harness;
use egui_stack::{
    DockArea, DockState, NodeIndex, TabDestination, TabViewer, tab_viewer::DropVerdict,
};

struct Viewer {
    refuse: bool,
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
    viewer: Viewer,
}

fn harness(refuse: bool) -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec!["First".to_owned(), "Second".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    let app = App {
        dock_state,
        viewer: Viewer {
            refuse,
            tab_rects: Vec::new(),
        },
    };
    Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            app.viewer.tab_rects.clear();
            DockArea::new(&mut app.dock_state).show_inside(ui, &mut app.viewer);
        },
        app,
    )
}

fn layout(app: &App) -> Vec<Vec<String>> {
    app.dock_state
        .main_surface()
        .iter()
        .filter_map(|node| node.tabs().map(<[String]>::to_vec))
        .collect()
}

/// Drags `First` onto the middle of the leaf holding `Third`, then releases it.
fn drag_first_onto_third(harness: &mut Harness<'_, App>) {
    harness.run();
    let rect_of = |harness: &Harness<'_, App>, title: &str| {
        harness
            .state()
            .viewer
            .tab_rects
            .iter()
            .find(|(tab, _)| tab == title)
            .map(|(_, rect)| *rect)
            .unwrap()
    };
    let start = rect_of(harness, "First").center();
    let third = rect_of(harness, "Third");
    // The middle of Third's leaf body, below its tab bar.
    let target = egui::pos2(
        harness.state().dock_state.main_surface()[NodeIndex(2)]
            .rect()
            .unwrap()
            .center()
            .x,
        third.bottom() + 150.0,
    );

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

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn an_allowed_drop_moves_the_tab() {
    let mut harness = harness(false);
    drag_first_onto_third(&mut harness);
    assert_eq!(
        layout(harness.state()),
        [
            vec!["Second".to_owned()],
            vec!["Third".to_owned(), "First".to_owned()]
        ]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_refused_drop_keeps_the_layout() {
    let mut harness = harness(true);
    let before = layout(harness.state());
    drag_first_onto_third(&mut harness);
    assert_eq!(layout(harness.state()), before);
}
