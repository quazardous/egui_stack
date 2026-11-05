//! A linear node holds more than two children: a separator resizes its two neighbours only, and
//! a tab dropped on a leaf's edge along the row joins the row instead of nesting a split.

use egui::{Id, Key, Modifiers, Pos2, Rect, Ui, WidgetText, accesskit::Role, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, NodeIndex, Style, TabViewer};

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

/// Three columns, `A`, `B` and `C`, of 400, 200 and 200 points' share in an 800-point dock.
fn harness() -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec!["A".to_owned(), "A2".to_owned()]);
    let tree = dock_state.main_surface_mut();
    let [_, b] = tree.split_right(NodeIndex::root(), 0.5, vec!["B".to_owned()]);
    tree.split_right(b, 0.5, vec!["C".to_owned()]);
    let app = App {
        dock_state,
        viewer: Viewer {
            tab_rects: Vec::new(),
        },
    };
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
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

/// The leaf rects of the root row, left to right.
fn columns(app: &App) -> Vec<Rect> {
    let tree = app.dock_state.main_surface();
    tree[NodeIndex::root()]
        .children()
        .iter()
        .map(|&child| tree[child].rect().unwrap())
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn three_columns_are_laid_out_by_share() {
    let harness = harness();
    let columns = columns(harness.state());
    assert_eq!(columns.len(), 3);
    let widths: Vec<f32> = columns.iter().map(Rect::width).collect();
    assert!((widths[0] - 2.0 * widths[1]).abs() < 3.0, "{widths:?}");
    assert!((widths[1] - widths[2]).abs() < 2.0, "{widths:?}");
    // Side by side, a separator between neighbours.
    assert!(columns[0].right() < columns[1].left());
    assert!(columns[1].right() < columns[2].left());
    assert_eq!(harness.query_all_by_role(Role::Splitter).count(), 2);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_separator_resizes_its_neighbours_only() {
    let mut harness = harness();
    let before = columns(harness.state());
    let second = harness.query_all_by_role(Role::Splitter).nth(1).unwrap();
    second.focus();
    harness.run();
    // One step: each column keeps at least `separator.extra` points.
    harness.key_press_modifiers(Modifiers::NONE, Key::ArrowLeft);
    harness.run();
    let after = columns(harness.state());
    assert_eq!(after[0], before[0], "the first column does not move");
    assert!(
        (after[1].width() - (before[1].width() - 16.0)).abs() < 1.5,
        "{before:?} {after:?}"
    );
    assert!(
        (after[2].width() - (before[2].width() + 16.0)).abs() < 1.5,
        "{before:?} {after:?}"
    );

    // Enter evens the pair, the first column still untouched.
    harness.key_press(Key::Enter);
    harness.run();
    let even = columns(harness.state());
    assert_eq!(even[0], before[0]);
    assert!((even[1].width() - even[2].width()).abs() < 1.5, "{even:?}");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_drop_on_the_edge_of_a_column_joins_the_row() {
    let mut harness = harness();
    let start = harness
        .state()
        .viewer
        .tab_rects
        .iter()
        .find(|(tab, _)| tab == "A2")
        .map(|(_, rect)| rect.center())
        .unwrap();
    // In the band along the right edge of `B`, the middle column.
    let target = |h: &Harness<'_, App>| -> Pos2 {
        let b = columns(h.state())[1];
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

    let tree = harness.state().dock_state.main_surface();
    let row: Vec<Vec<String>> = tree[NodeIndex::root()]
        .children()
        .iter()
        .map(|&child| tree[child].tabs().expect("a leaf").to_vec())
        .collect();
    assert_eq!(
        row,
        [
            vec!["A".to_owned()],
            vec!["B".to_owned()],
            vec!["A2".to_owned()],
            vec!["C".to_owned()],
        ],
        "four columns, no nested split"
    );
    let shares = tree[NodeIndex::root()].get_linear().unwrap().shares();
    assert!((shares[1] - shares[2]).abs() < 1e-6, "{shares:?}");
}
