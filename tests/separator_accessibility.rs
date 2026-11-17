//! Separators for keyboard and assistive technology users: a `DockArea` separator and a
//! `LayoutArea` sash are Tab stops, splitters named after what they resize ("Resize between
//! Left and Right"), whose value is a range with a minimum, a maximum and a step, set or stepped
//! through accesskit actions.

use egui::{Id, Key, Ui, WidgetText, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::layout::{Content, Dir, Layout, LayoutArea, Size, Zone, ZoneName, ZoneViewer};
use egui_stack::{DockArea, DockState, Labels, LinearDir, Node, NodeIndex, TabViewer};

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

struct DockApp {
    dock_state: DockState<String>,
    labels: Labels,
}

fn dock_harness(dock_state: DockState<String>, labels: Labels) -> Harness<'static, DockApp> {
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut DockApp| {
            DockArea::new(&mut app.dock_state)
                .labels(app.labels.clone())
                .show_inside(ui, &mut Viewer);
        },
        DockApp { dock_state, labels },
    );
    harness.run();
    harness
}

/// `Left | Right`.
fn left_right() -> DockState<String> {
    let mut dock_state = DockState::new(vec!["Left".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Right".to_owned()]);
    dock_state
}

fn split_fraction(app: &DockApp) -> f32 {
    match &app.dock_state.main_surface()[NodeIndex::root()] {
        Node::Linear(split) => split.boundary(0),
        node => panic!("the root is a split, not {node:?}"),
    }
}

/// Presses Tab until a splitter has the focus, at most `limit` times; returns how many it took.
fn tab_to_splitter<S>(harness: &mut Harness<'_, S>, limit: usize) -> Option<usize> {
    for pressed in 1..=limit {
        harness.key_press(Key::Tab);
        harness.run();
        if harness
            .query_all_by_role(Role::Splitter)
            .any(|node| node.is_focused())
        {
            return Some(pressed);
        }
    }
    None
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dock_separator_is_reached_with_tab_and_moved_with_the_arrows() {
    let mut harness = dock_harness(left_right(), Labels::default());
    assert!(
        tab_to_splitter(&mut harness, 40).is_some(),
        "Tab reaches the separator"
    );
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(split_fraction(harness.state()) > 0.5);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dock_separator_is_named_after_its_neighbours() {
    let harness = dock_harness(left_right(), Labels::default());
    let separator = harness.get_by_role(Role::Splitter);
    assert_eq!(
        separator.accesskit_node().label().as_deref(),
        Some("Resize between Left and Right")
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_neighbour_holding_a_split_is_named_after_its_first_leafs_active_tab() {
    // `Left | (Top / Bottom)`: the right neighbour of the first separator is a split.
    let mut dock_state = left_right();
    let surface = dock_state.main_surface_mut();
    let (right, _) = surface.find_tab(&"Right".to_owned()).unwrap();
    surface.split_below(right, 0.5, vec!["Bottom".to_owned()]);
    let harness = dock_harness(dock_state, Labels::default());
    harness.get_by_role_and_label(Role::Splitter, "Resize between Left and Right");
    harness.get_by_role_and_label(Role::Splitter, "Resize between Right and Bottom");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_separator_name_is_a_settable_label() {
    let mut labels = Labels::default();
    labels.separator_between = "Redimensionner entre {first} et {second}".into();
    let harness = dock_harness(left_right(), labels);
    harness.get_by_role_and_label(Role::Splitter, "Redimensionner entre Left et Right");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dock_separator_is_a_range_set_and_stepped_by_accesskit() {
    let mut harness = dock_harness(left_right(), Labels::default());
    let width = match &harness.state().dock_state.main_surface()[NodeIndex::root()] {
        Node::Linear(split) if split.dir == LinearDir::Horizontal => split.rect.width(),
        node => panic!("{node:?}"),
    };
    {
        let separator = harness.get_by_role(Role::Splitter);
        let node = separator.accesskit_node();
        assert_eq!(node.numeric_value(), Some(0.5));
        let (min, max) = (
            node.min_numeric_value().unwrap(),
            node.max_numeric_value().unwrap(),
        );
        assert!(
            0.0 < min && min < 0.5 && 0.5 < max && max < 1.0,
            "{min} {max}"
        );
        let step = node.numeric_value_step().unwrap();
        assert!((step - f64::from(16.0 / width)).abs() < 1e-6, "{step}");
    }
    harness.get_by_role(Role::Splitter).set_value_accesskit(0.3);
    harness.run();
    assert!((split_fraction(harness.state()) - 0.3).abs() < 1e-6);
    harness.get_by_role(Role::Splitter).increment_accesskit();
    harness.run();
    assert!((split_fraction(harness.state()) - (0.3 + 16.0 / width)).abs() < 1e-4);
}

struct Zones;

impl ZoneViewer for Zones {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        ui.label(format!("Body of {zone}"));
    }

    fn title(&mut self, zone: &ZoneName) -> String {
        format!("The {zone} area")
    }
}

fn layout_harness() -> Harness<'static, Layout> {
    let layout = Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::leaf("left", Content::App).with(|c| {
                c.min = Some(Size::Points(100.0));
                c.default = Some(Size::Points(200.0));
            }),
            Zone::leaf("centre", Content::App).with(|c| c.grow = true),
        ],
    ));
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, layout: &mut Layout| {
            LayoutArea::new(layout).show_inside(ui, &mut Zones);
        },
        layout,
    );
    harness.run();
    harness
}

fn left_size(harness: &Harness<'_, Layout>) -> Option<f32> {
    harness.state().zone("left").unwrap().state.size
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_layout_sash_is_reached_with_tab_and_moved_with_the_arrows() {
    let mut harness = layout_harness();
    assert!(
        tab_to_splitter(&mut harness, 40).is_some(),
        "Tab reaches the sash"
    );
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(left_size(&harness), Some(216.0));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_layout_sash_is_a_range_in_points_set_and_stepped_by_accesskit() {
    let mut harness = layout_harness();
    {
        let sash = harness.get_by_role_and_label(Role::Splitter, "Resize The left area");
        let node = sash.accesskit_node();
        assert_eq!(node.numeric_value(), Some(200.0));
        assert_eq!(node.min_numeric_value(), Some(100.0));
        assert!(node.max_numeric_value().unwrap() > 200.0);
        assert_eq!(node.numeric_value_step(), Some(16.0));
    }
    harness
        .get_by_role(Role::Splitter)
        .set_value_accesskit(300.0);
    harness.run();
    assert_eq!(left_size(&harness), Some(300.0));
    harness.get_by_role(Role::Splitter).decrement_accesskit();
    harness.run();
    assert_eq!(left_size(&harness), Some(284.0));
}
