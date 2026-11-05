//! Buttons and separators are exposed to AccessKit with roles and names from [`Labels`].
//!
//! The harness's accessibility check stays on: every input widget it finds must have a name.

use egui::{Id, Ui, WidgetText, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::{DockArea, DockState, Labels, LinearDir, Node, NodeIndex, NodePath, TabViewer};

#[derive(Default)]
struct Viewer {
    added: usize,
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

    fn on_add(&mut self, _path: NodePath) {
        self.added += 1;
    }
}

struct App {
    dock_state: DockState<String>,
    viewer: Viewer,
    labels: Labels,
}

fn split_dock() -> DockState<String> {
    let mut dock_state = DockState::new(vec!["First".to_owned(), "Second".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    dock_state
}

fn harness(dock_state: DockState<String>, labels: Labels) -> Harness<'static, App> {
    let app = App {
        dock_state,
        viewer: Viewer::default(),
        labels,
    };
    Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            DockArea::new(&mut app.dock_state)
                .show_add_buttons(true)
                .hidable_tab_bars(true)
                .labels(app.labels.clone())
                .show_inside(ui, &mut app.viewer);
        },
        app,
    )
}

fn split_fraction(app: &App) -> f32 {
    match &app.dock_state.main_surface()[NodeIndex::root()] {
        Node::Linear(split) if split.dir == LinearDir::Horizontal => split.boundary(0),
        node => panic!("the root is a horizontal split, not {node:?}"),
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn buttons_are_named() {
    let mut harness = harness(split_dock(), Labels::default());
    harness.run();

    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Button, "Close tab")
            .count(),
        3
    );
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Button, "Close all tabs")
            .count(),
        2
    );
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Button, "Add tab")
            .count(),
        2
    );

    let collapse = |harness: &Harness<'_, App>| {
        harness
            .query_all_by_role_and_label(Role::Button, "Collapse")
            .map(|node| node.accesskit_node().data().is_expanded())
            .collect::<Vec<_>>()
    };
    assert_eq!(collapse(&harness), [Some(true), Some(true)]);

    harness
        .query_all_by_role_and_label(Role::Button, "Add tab")
        .next()
        .unwrap()
        .click();
    harness.run();
    assert_eq!(harness.state().viewer.added, 1);

    harness
        .query_all_by_role_and_label(Role::Button, "Collapse")
        .next()
        .unwrap()
        .click();
    harness.run();
    assert!(collapse(&harness).contains(&Some(false)));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn labels_are_settable() {
    let mut labels = Labels::default();
    labels.close_tab = "Fermer l'onglet".into();
    labels.separator = "Redimensionner".into();
    let mut harness = harness(split_dock(), labels);
    harness.run();

    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Button, "Fermer l'onglet")
            .count(),
        3
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Close tab")
            .is_none()
    );
    harness.get_by_role_and_label(Role::Splitter, "Redimensionner");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_window_can_be_minimized_and_restored() {
    let mut dock_state = DockState::new(vec!["Main".to_owned()]);
    let surface = dock_state.add_window(vec!["Floating".to_owned()]);
    dock_state
        .get_window_state_mut(surface)
        .unwrap()
        .set_position(egui::pos2(300.0, 150.0));
    let mut harness = harness(dock_state, Labels::default());
    harness.run();

    // The window's collapse button is the one nearest the window's corner.
    let corner = egui::pos2(300.0, 150.0);
    let collapse = harness
        .query_all_by_role_and_label(Role::Button, "Collapse")
        .min_by(|a, b| {
            let a = a.rect().center().distance(corner);
            let b = b.rect().center().distance(corner);
            a.total_cmp(&b)
        })
        .unwrap();
    collapse.click_secondary();
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Minimize window")
        .click();
    harness.run();

    harness
        .get_by_role_and_label(Role::Button, "Restore window")
        .click();
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Restore window")
            .is_none()
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn separators_are_splitters() {
    let mut harness = harness(split_dock(), Labels::default());
    harness.run();

    let numeric = |harness: &Harness<'_, App>| {
        harness
            .get_by_role_and_label(Role::Splitter, "Resize")
            .accesskit_node()
            .numeric_value()
            .unwrap()
    };
    assert_eq!(numeric(&harness), 0.5);
    assert_eq!(split_fraction(harness.state()), 0.5);

    harness
        .get_by_role_and_label(Role::Splitter, "Resize")
        .increment_accesskit();
    harness.run();
    let increased = split_fraction(harness.state());
    assert!(increased > 0.5, "{increased}");
    assert_eq!(numeric(&harness), f64::from(increased));

    harness
        .get_by_role_and_label(Role::Splitter, "Resize")
        .decrement_accesskit();
    harness.run();
    assert!((split_fraction(harness.state()) - 0.5).abs() < 1e-6);

    harness
        .get_by_role_and_label(Role::Splitter, "Resize")
        .set_value_accesskit(0.25);
    harness.run();
    assert!((split_fraction(harness.state()) - 0.25).abs() < 1e-6);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_hidden_tab_bar_can_be_shown() {
    let mut dock_state = DockState::new(vec!["Only".to_owned()]);
    dock_state.main_surface_mut()[NodeIndex::root()].set_tab_bar_hidden(true);
    let mut harness = harness(dock_state, Labels::default());
    harness.run();

    harness
        .get_by_role_and_label(Role::Button, "Show tab bar")
        .click();
    harness.run();
    assert!(!harness.state().dock_state.main_surface()[NodeIndex::root()].is_tab_bar_hidden());
}
