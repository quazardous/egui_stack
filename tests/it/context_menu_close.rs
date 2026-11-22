//! The tab context menu's Close asks the viewer once.

use egui::{Event, Id, Modifiers, PointerButton, Rect, Ui, WidgetText};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, TabViewer, tab_viewer::OnCloseResponse};

#[derive(Default)]
struct Viewer {
    on_close_calls: usize,
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
        ui.label(tab.as_str());
    }

    fn on_tab_button(&mut self, tab: &mut String, response: &egui::Response) {
        self.tab_rects.push((tab.clone(), response.rect));
    }

    fn on_close(&mut self, _tab: &mut String) -> OnCloseResponse {
        self.on_close_calls += 1;
        OnCloseResponse::Close
    }
}

struct App {
    dock_state: DockState<String>,
    viewer: Viewer,
}

fn harness() -> Harness<'static, App> {
    let app = App {
        dock_state: DockState::new(vec!["First".to_owned(), "Second".to_owned()]),
        viewer: Viewer::default(),
    };
    Harness::builder()
        .with_size([400.0, 300.0])
        // Buttons get accessible names in a later change; this test only needs "Close".
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, app: &mut App| {
                app.viewer.tab_rects.clear();
                DockArea::new(&mut app.dock_state).show_inside(ui, &mut app.viewer);
            },
            app,
        )
}

fn right_click(harness: &mut Harness<'_, App>, title: &str) {
    let rect = harness
        .state()
        .viewer
        .tab_rects
        .iter()
        .find(|(tab, _)| tab == title)
        .map(|(_, rect)| *rect)
        .expect("the tab is shown");
    let pos = rect.center();
    harness.event(Event::PointerMoved(pos));
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn context_menu_close_asks_the_viewer_once() {
    let mut harness = harness();
    harness.run();

    for (closed, title) in ["First", "Second"].into_iter().enumerate() {
        right_click(&mut harness, title);
        harness.get_by_label("Close").click();
        harness.run();

        let app = harness.state();
        assert_eq!(app.viewer.on_close_calls, closed + 1, "closing {title}");
        assert!(
            app.dock_state.iter_all_tabs().all(|(_, tab)| tab != title),
            "{title} is closed"
        );
    }
}
