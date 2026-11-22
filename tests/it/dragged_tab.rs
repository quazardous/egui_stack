//! `DockArea::dragged_tab` reports the tab being dragged and the pointer, then nothing once dropped.

use egui::{Id, Ui, WidgetText, accesskit::Role, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, DraggedTab, NodeIndex, SurfaceIndex, TabIndex, TabViewer};

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

struct App {
    dock_state: DockState<String>,
    /// What `DockArea::dragged_tab` said after each frame.
    seen: Option<DraggedTab>,
}

fn harness() -> Harness<'static, App> {
    let app = App {
        dock_state: DockState::new(vec!["First".to_owned(), "Second".to_owned()]),
        seen: None,
    };
    Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        |ui, app: &mut App| {
            let id = Id::unique("dock");
            DockArea::new(&mut app.dock_state)
                .id(id)
                .show_inside(ui, &mut Viewer);
            app.seen = DockArea::dragged_tab(ui.ctx(), id);
        },
        app,
    )
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_dragged_tab_and_pointer_are_public() {
    let mut harness = harness();
    harness.run();
    assert_eq!(harness.state().seen, None);

    let start = harness
        .get_by_role_and_label(Role::Tab, "Second")
        .rect()
        .center();
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    let mut pointer = start;
    for _ in 0..5 {
        pointer += vec2(0.0, 20.0);
        harness.hover_at(pointer);
        harness.step();
    }

    let seen = harness.state().seen.expect("a tab is being dragged");
    assert_eq!(seen.path.surface, SurfaceIndex::main());
    assert_eq!(seen.path.node, NodeIndex::root());
    assert_eq!(seen.path.tab, TabIndex(1));
    assert_eq!(seen.pointer, pointer);

    harness.drop_at(pointer);
    harness.run_steps(3);
    assert_eq!(harness.state().seen, None);
}
