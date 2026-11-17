//! A tab group dragged by a handle other than its tab bar, for an editor whose tab bar is
//! hidden: a widget of the tab's body made a handle with `DockArea::drag_group_by`, or the strip
//! left at the top of a hidden tab bar's leaf with `DockArea::hidden_tab_bar_drags_group`.
//! Either moves the whole group, its tabs in order and its active tab kept; without them nothing
//! changes.

use egui::{Id, Pos2, Rect, Sense, Ui, UiBuilder, WidgetText, pos2, vec2};
use egui_kittest::Harness;
use egui_stack::{
    DockArea, DockState, DraggedNode, NodeIndex, NodePath, Style, SurfaceIndex, TabIndex, TabViewer,
};

/// Each body starts with a 20-point header; with `handles`, the header is a group drag handle.
struct Viewer<'a> {
    handles: bool,
    header: &'a mut Option<Rect>,
    handle_dragged: &'a mut bool,
}

impl TabViewer for Viewer<'_> {
    type Tab = String;

    fn id(&mut self, tab: &mut String) -> Id {
        Id::unique(tab.as_str())
    }

    fn title(&mut self, tab: &mut String) -> WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut String) {
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::drag());
        if tab == "C" {
            *self.header = Some(rect);
            if self.handles {
                *self.handle_dragged |= DockArea::drag_group_by(ui, &response);
            }
        }
        ui.label(format!("Body of {tab}"));
    }
}

#[derive(Clone, Copy, Default)]
struct Options {
    handles: bool,
    draggable: bool,
    hidden_bar: bool,
    strip_drags_group: bool,
}

struct App {
    dock_state: DockState<String>,
    options: Options,
    header: Option<Rect>,
    handle_dragged: bool,
    seen: Option<DraggedNode>,
}

const ID: &str = "dock";

/// `A` on the left; `B`, `C`, `D` on the right, `C` shown. The dock takes the left 600 points
/// of an 800 × 400 screen.
fn harness(options: Options) -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec!["A".to_owned()]);
    let [_, right] = dock_state.main_surface_mut().split_right(
        NodeIndex::root(),
        0.5,
        vec!["B".to_owned(), "C".to_owned(), "D".to_owned()],
    );
    let right = dock_state
        .leaf_mut(NodePath::new(SurfaceIndex::main(), right))
        .unwrap();
    right.active = TabIndex(1);
    right.tab_bar_hidden = options.hidden_bar;
    let app = App {
        dock_state,
        options,
        header: None,
        handle_dragged: false,
        seen: None,
    };
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            let dock = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0));
            let mut style = Style::from_egui(ui.style());
            style.tab.tab_body.hidden_tab_bar_drag_height = Some(8.0);
            let mut viewer = Viewer {
                handles: app.options.handles,
                header: &mut app.header,
                handle_dragged: &mut app.handle_dragged,
            };
            ui.scope_builder(UiBuilder::new().max_rect(dock), |ui| {
                DockArea::new(&mut app.dock_state)
                    .id(Id::unique(ID))
                    .style(style)
                    .draggable_tabs(app.options.draggable)
                    .hidable_tab_bars(true)
                    .hidden_tab_bar_drags_group(app.options.strip_drags_group)
                    .show_inside(ui, &mut viewer);
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

/// Presses at `start`, moves down then to the middle of `A`'s leaf, and lets go. Returns whether
/// a group drag was reported on the way.
fn drag_onto_a(harness: &mut Harness<'_, App>, start: Pos2) -> bool {
    let leaf = leaf_rect(harness, "A");
    let target = pos2(leaf.center().x, leaf.center().y + 30.0);
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=5 {
        harness.hover_at(start + vec2(0.0, 10.0 * step as f32));
        harness.step();
    }
    let mut seen = false;
    for _ in 0..5 {
        harness.hover_at(target);
        harness.step();
        seen |= harness.state().seen.is_some();
    }
    harness.drop_at(target);
    harness.run_steps(5);
    assert_eq!(harness.state().seen, None);
    seen
}

/// The leaves of the main surface with their active tab.
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

fn header_center(harness: &Harness<'_, App>) -> Pos2 {
    harness.state().header.expect("C's body is shown").center()
}

/// The strip at the top of the right leaf, whose tab bar is hidden, clear of its show button.
fn strip_center(harness: &Harness<'_, App>) -> Pos2 {
    let leaf = leaf_rect(harness, "B");
    pos2(leaf.center().x, leaf.top() + 4.0)
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_body_handle_drags_the_whole_group() {
    let mut harness = harness(Options {
        handles: true,
        draggable: true,
        hidden_bar: true,
        ..Options::default()
    });
    let start = header_center(&harness);
    assert!(drag_onto_a(&mut harness, start), "the group was dragged");
    assert!(harness.state().handle_dragged);
    assert_eq!(leaves(harness.state()), [(names(&["A", "B", "C", "D"]), 2)]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_body_handle_works_with_the_tab_bar_shown() {
    let mut harness = harness(Options {
        handles: true,
        draggable: true,
        ..Options::default()
    });
    let start = header_center(&harness);
    assert!(drag_onto_a(&mut harness, start));
    assert_eq!(leaves(harness.state()), [(names(&["A", "B", "C", "D"]), 2)]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_body_widget_not_made_a_handle_drags_nothing() {
    let mut harness = harness(Options {
        draggable: true,
        hidden_bar: true,
        ..Options::default()
    });
    let before = leaves(harness.state());
    let start = header_center(&harness);
    assert!(!drag_onto_a(&mut harness, start));
    assert_eq!(leaves(harness.state()), before);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_body_handle_drags_nothing_when_tabs_are_not_draggable() {
    let mut harness = harness(Options {
        handles: true,
        hidden_bar: true,
        ..Options::default()
    });
    let before = leaves(harness.state());
    let start = header_center(&harness);
    assert!(!drag_onto_a(&mut harness, start));
    assert!(!harness.state().handle_dragged);
    assert_eq!(leaves(harness.state()), before);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_hidden_bar_strip_drags_the_group_when_asked() {
    let mut harness = harness(Options {
        draggable: true,
        hidden_bar: true,
        strip_drags_group: true,
        ..Options::default()
    });
    let start = strip_center(&harness);
    assert!(drag_onto_a(&mut harness, start));
    assert_eq!(leaves(harness.state()), [(names(&["A", "B", "C", "D"]), 2)]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_hidden_bar_strip_still_drags_the_active_tab_by_default() {
    let mut harness = harness(Options {
        draggable: true,
        hidden_bar: true,
        ..Options::default()
    });
    let start = strip_center(&harness);
    assert!(!drag_onto_a(&mut harness, start));
    let all = leaves(harness.state());
    assert!(all.contains(&(names(&["A", "C"]), 1)), "{all:?}");
    assert!(all.contains(&(names(&["B", "D"]), 0)) || all.contains(&(names(&["B", "D"]), 1)));
}
