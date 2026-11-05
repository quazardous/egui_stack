//! A grid lays out its children in rows and columns: as many columns as the square root of
//! their number, rounded up. A separator on a column line resizes the two columns beside it, in
//! every row; a view added at run time or dropped on a grid child's edge joins the grid.

use egui::{Id, Key, Modifiers, Pos2, Rect, Ui, WidgetText, accesskit::Role, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, GridLayout, Node, NodeIndex, Style, TabViewer};

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

/// A grid of `count` views, `A` (with a second tab, `A2`), `B`, `C` and so on, in an 800 by
/// 600 point dock.
fn harness(count: usize) -> Harness<'static, App> {
    let names: Vec<String> = (0..count)
        .map(|i| char::from(b'A' + i as u8).to_string())
        .collect();
    let mut dock_state = DockState::new(vec![names[0].clone(), "A2".to_owned()]);
    let tree = dock_state.main_surface_mut();
    let mut last = NodeIndex::root();
    for name in &names[1..] {
        last = tree.split_right(last, 0.5, vec![name.clone()])[1];
    }
    assert!(tree.arrange_as_grid(NodeIndex::root(), GridLayout::Auto));
    let app = App {
        dock_state,
        viewer: Viewer {
            tab_rects: Vec::new(),
        },
    };
    let mut harness = Harness::builder().with_size([800.0, 600.0]).build_ui_state(
        |ui, app: &mut App| {
            app.viewer.tab_rects.clear();
            DockArea::new(&mut app.dock_state)
                .style(Style::from_egui(ui.style()))
                .show_inside(ui, &mut app.viewer);
        },
        app,
    );
    harness.run();
    harness
}

/// The rects of the grid's children, in their order.
fn cells(app: &App) -> Vec<Rect> {
    let tree = app.dock_state.main_surface();
    tree[NodeIndex::root()]
        .children()
        .iter()
        .map(|&child| tree[child].rect().unwrap())
        .collect()
}

/// The first tab of each of the grid's children, in their order.
fn names(app: &App) -> Vec<String> {
    let tree = app.dock_state.main_surface();
    tree[NodeIndex::root()]
        .children()
        .iter()
        .map(|&child| tree[child].tabs().expect("a leaf")[0].clone())
        .collect()
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 2.0
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn four_views_make_two_rows_of_two() {
    let harness = harness(4);
    let cells = cells(harness.state());
    assert_eq!(cells.len(), 4);
    // Row by row: A B / C D.
    assert!(near(cells[0].top(), cells[1].top()), "{cells:?}");
    assert!(near(cells[2].top(), cells[3].top()), "{cells:?}");
    assert!(cells[0].bottom() < cells[2].top(), "{cells:?}");
    assert!(cells[0].right() < cells[1].left(), "{cells:?}");
    assert!(near(cells[0].left(), cells[2].left()), "{cells:?}");
    assert!(near(cells[0].width(), cells[1].width()), "{cells:?}");
    assert!(near(cells[0].height(), cells[2].height()), "{cells:?}");
    // One column line and one row line.
    assert_eq!(harness.query_all_by_role(Role::Splitter).count(), 2);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn five_views_make_three_columns_and_a_short_row() {
    let harness = harness(5);
    let cells = cells(harness.state());
    // A B C / D E, the last cell empty.
    assert!(near(cells[0].top(), cells[2].top()), "{cells:?}");
    assert!(cells[1].right() < cells[2].left(), "{cells:?}");
    assert!(cells[2].bottom() < cells[3].top(), "{cells:?}");
    assert!(near(cells[3].left(), cells[0].left()), "{cells:?}");
    assert!(near(cells[4].left(), cells[1].left()), "{cells:?}");
    assert!(near(cells[4].width(), cells[1].width()), "{cells:?}");
    assert!(near(cells[0].width(), cells[2].width()), "{cells:?}");
    // Two column lines and one row line.
    assert_eq!(harness.query_all_by_role(Role::Splitter).count(), 3);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_column_line_resizes_its_columns_in_every_row() {
    let mut harness = harness(4);
    let before = cells(harness.state());
    // The column lines come first, then the row lines.
    let column_line = harness.query_all_by_role(Role::Splitter).next().unwrap();
    column_line.focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::NONE, Key::ArrowRight);
    harness.run();
    let after = cells(harness.state());
    for (left, right) in [(0, 1), (2, 3)] {
        assert!(
            near(after[left].width(), before[left].width() + 16.0),
            "{before:?} {after:?}"
        );
        assert!(
            near(after[right].width(), before[right].width() - 16.0),
            "{before:?} {after:?}"
        );
    }
    // The rows keep their height.
    assert_eq!(after[0].height(), before[0].height());
    assert_eq!(after[2].top(), before[2].top());

    // Enter evens the pair again.
    harness.key_press(Key::Enter);
    harness.run();
    let even = cells(harness.state());
    assert!(near(even[0].width(), even[1].width()), "{even:?}");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_view_added_at_run_time_joins_the_grid() {
    let mut harness = harness(4);
    harness
        .state_mut()
        .dock_state
        .main_surface_mut()
        .insert_child(NodeIndex::root(), 4, Node::leaf("E".to_owned()));
    harness.run();
    let cells = cells(harness.state());
    assert_eq!(cells.len(), 5);
    // Three columns now: A B C / D E.
    assert!(near(cells[0].top(), cells[2].top()), "{cells:?}");
    assert!(cells[2].bottom() < cells[3].top(), "{cells:?}");
    assert!(near(cells[4].left(), cells[1].left()), "{cells:?}");
    assert_eq!(harness.query_all_by_role(Role::Splitter).count(), 3);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_drop_on_a_grid_childs_edge_joins_the_grid() {
    let mut harness = harness(4);
    let start = harness
        .state()
        .viewer
        .tab_rects
        .iter()
        .find(|(tab, _)| tab == "A2")
        .map(|(_, rect)| rect.center())
        .unwrap();
    // In the band along the right edge of `B`.
    let target = |h: &Harness<'_, App>| -> Pos2 {
        let b = cells(h.state())[1];
        pos2(b.right() - 10.0, b.center().y)
    };
    let target = target(&harness);
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

    assert_eq!(names(harness.state()), ["A", "B", "A2", "C", "D"]);
    let tree = harness.state().dock_state.main_surface();
    let grid = tree[NodeIndex::root()].get_grid().expect("still a grid");
    assert_eq!((grid.columns(), grid.rows()), (3, 2));
}
