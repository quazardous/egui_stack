//! A focused separator moves with the arrow keys along its axis, further with Shift, and Enter
//! resets it.

use egui::{Id, Key, Modifiers, Ui, WidgetText, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, LinearDir, Node, NodeIndex, TabViewer};

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
    let mut dock_state = DockState::new(vec!["Left".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Right".to_owned()]);
    Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, dock_state: &mut DockState<String>| {
            DockArea::new(dock_state).show_inside(ui, &mut Viewer);
        },
        dock_state,
    )
}

/// The split's fraction and its width in points.
fn split(dock_state: &DockState<String>) -> (f32, f32) {
    match &dock_state.main_surface()[NodeIndex::root()] {
        Node::Linear(split) if split.dir == LinearDir::Horizontal => {
            (split.boundary(0), split.rect.width())
        }
        node => panic!("the root is a horizontal split, not {node:?}"),
    }
}

fn press(harness: &mut Harness<'_, DockState<String>>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn arrows_move_a_focused_separator_and_enter_resets_it() {
    let mut harness = harness();
    harness.run();
    harness.get_by_role(Role::Splitter).focus();
    harness.run();
    assert!(harness.get_by_role(Role::Splitter).is_focused());

    let (start, width) = split(harness.state());
    assert_eq!(start, 0.5);

    press(&mut harness, Modifiers::NONE, Key::ArrowRight);
    let (fraction, _) = split(harness.state());
    assert!(
        (fraction - (start + 16.0 / width)).abs() < 1e-4,
        "{fraction}"
    );

    press(&mut harness, Modifiers::SHIFT, Key::ArrowLeft);
    let (fraction, _) = split(harness.state());
    assert!(
        (fraction - (start - 48.0 / width)).abs() < 1e-4,
        "{fraction}"
    );

    // Arrows across the separator's axis do not move it.
    press(&mut harness, Modifiers::NONE, Key::ArrowDown);
    assert_eq!(split(harness.state()).0, fraction);

    // They may move the focus away; take it back.
    harness.get_by_role(Role::Splitter).focus();
    harness.run();
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert!((split(harness.state()).0 - 0.5).abs() < 1e-6);
}
