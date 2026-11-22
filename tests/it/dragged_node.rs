//! A whole tab group dragged by the empty part of its tab bar, as an editor group is dragged by
//! its tab bar: all its tabs move together, in order, its active tab kept; the middle of another
//! leaf takes them as tabs, an edge band splits that leaf, outside the dock they open in a new
//! window, and onto itself nothing moves. `DockArea::dragged_node` reports the drag.

use egui::{Id, Pos2, Rect, Ui, UiBuilder, WidgetText, pos2, vec2};
use egui_kittest::Harness;
use egui_stack::{
    DockArea, DockState, DraggedNode, NodeIndex, NodePath, SurfaceIndex, TabIndex, TabViewer,
};

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
    seen: Option<DraggedNode>,
}

const ID: &str = "dock";

/// `A` on the left; `B`, `C`, `D` on the right, `C` shown. The dock takes the left 600 points
/// of an 800 × 400 screen, so the rest is outside it.
fn harness() -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec!["A".to_owned()]);
    let [_, right] = dock_state.main_surface_mut().split_right(
        NodeIndex::root(),
        0.5,
        vec!["B".to_owned(), "C".to_owned(), "D".to_owned()],
    );
    dock_state
        .leaf_mut(NodePath::new(SurfaceIndex::main(), right))
        .unwrap()
        .active = TabIndex(1);
    let app = App {
        dock_state,
        seen: None,
    };
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            let dock = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0));
            ui.scope_builder(UiBuilder::new().max_rect(dock), |ui| {
                DockArea::new(&mut app.dock_state)
                    .id(Id::unique(ID))
                    .show_inside(ui, &mut Viewer);
            });
            app.seen = DockArea::dragged_node(ui.ctx(), Id::unique(ID));
        },
        app,
    );
    harness.run();
    harness
}

fn leaf_rect(harness: &Harness<'_, App>, tab: &str) -> Rect {
    let surface = harness.state().dock_state.main_surface();
    let (node, _) = surface.find_tab(&tab.to_owned()).unwrap();
    surface[node].rect().unwrap()
}

/// The empty part of the right leaf's tab bar, near its right end.
fn right_bar_tail(harness: &Harness<'_, App>) -> Pos2 {
    let leaf = leaf_rect(harness, "B");
    pos2(leaf.right() - 20.0, leaf.top() + 10.0)
}

/// Drags the right group by its bar to where `target` says, holds it there, and lets it go.
fn drag_group_to(harness: &mut Harness<'_, App>, target: impl Fn(&Harness<'_, App>) -> Pos2) {
    let start = right_bar_tail(harness);
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
    let seen = harness.state().seen.expect("the group is being dragged");
    assert_eq!(seen.pointer, target);
    harness.drop_at(target);
    harness.run_steps(5);
    assert_eq!(harness.state().seen, None);
}

/// The leaves of the main surface, left to right and top to bottom, with their active tab.
fn leaves(app: &App) -> Vec<(Vec<String>, usize)> {
    app.dock_state
        .main_surface()
        .iter()
        .filter_map(|node| node.get_leaf())
        .map(|leaf| (leaf.tabs.clone(), leaf.active.0))
        .collect()
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_middle_of_a_leaf_takes_all_the_tabs_in_order_and_the_active_one() {
    let mut harness = harness();
    drag_group_to(&mut harness, |h| {
        let leaf = leaf_rect(h, "A");
        pos2(leaf.center().x, leaf.center().y + 30.0)
    });
    assert_eq!(leaves(harness.state()), [(names(&["A", "B", "C", "D"]), 2)]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn an_edge_band_splits_the_leaf_with_the_whole_group() {
    let mut harness = harness();
    drag_group_to(&mut harness, |h| {
        let leaf = leaf_rect(h, "A");
        pos2(leaf.center().x, leaf.bottom() - 10.0)
    });
    let all = leaves(harness.state());
    assert_eq!(all.len(), 2);
    assert!(all.contains(&(names(&["A"]), 0)));
    assert!(all.contains(&(names(&["B", "C", "D"]), 1)));
    let a = leaf_rect(&harness, "A");
    let b = leaf_rect(&harness, "B");
    assert!(b.top() > a.bottom() - 1.0, "the group below A");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn outside_the_dock_the_group_opens_in_a_window() {
    let mut harness = harness();
    drag_group_to(&mut harness, |_| pos2(700.0, 200.0));
    let dock = &harness.state().dock_state;
    let b = dock.find_tab(&"B".to_owned()).unwrap();
    assert_ne!(b.surface, SurfaceIndex::main());
    let leaf = dock.leaf(b.node_path()).unwrap();
    assert_eq!(leaf.tabs, names(&["B", "C", "D"]));
    assert_eq!(leaf.active, TabIndex(1));
    assert_eq!(leaves(harness.state()), [(names(&["A"]), 0)]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn onto_itself_nothing_moves() {
    let mut harness = harness();
    let before = leaves(harness.state());
    drag_group_to(&mut harness, |h| {
        let leaf = leaf_rect(h, "B");
        pos2(leaf.center().x, leaf.center().y + 30.0)
    });
    assert_eq!(leaves(harness.state()), before);
}

#[test]
fn move_node_into_an_empty_surface_and_onto_itself() {
    let mut dock = DockState::new(vec!["a".to_owned(), "b".to_owned()]);
    let root = NodePath::new(SurfaceIndex::main(), NodeIndex::root());
    dock.leaf_mut(root).unwrap().active = TabIndex(1);
    dock.move_node(
        root,
        egui_stack::TabDestination::Node(root, egui_stack::TabInsert::Append),
    );
    let leaf = dock.leaf(root).unwrap();
    assert_eq!(leaf.tabs, names(&["a", "b"]));
    let window = dock.add_window(vec!["w".to_owned()]);
    dock.move_node(
        root,
        egui_stack::TabDestination::Node(
            NodePath::new(window, NodeIndex::root()),
            egui_stack::TabInsert::Insert(TabIndex(0)),
        ),
    );
    let leaf = dock.leaf(NodePath::new(window, NodeIndex::root())).unwrap();
    assert_eq!(leaf.tabs, names(&["a", "b", "w"]));
    assert_eq!(leaf.active, TabIndex(1));
    assert!(dock.main_surface().is_empty());
}
