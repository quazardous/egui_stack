//! A `DockSpace`: several named docks, each drawn by its own `DockArea::dock` call, sharing one
//! drag. A tab dragged from one dock to another gets the same zones as inside one dock: the
//! middle of a leaf, its four edge bands, a slot in its tab bar; the viewer's veto and each
//! dock's rules hold across docks.

use std::{cell::RefCell, fmt::Write as _, sync::Mutex};

use egui::epaint::ClippedShape;
use egui::{Id, Pos2, Rect, Shape, Ui, UiBuilder, WidgetText, pos2, vec2};
use egui_kittest::Harness;
use egui_stack::{
    AllowedSplits, DockArea, DockDrop, DockPainter, DockRules, DockSpace, DockState, DropPaint,
    NodeIndex, NodePath, Split, StylePainter, SurfaceIndex, TabDestination, TabInsert, TabViewer,
    Tree, tab_viewer::DropVerdict,
};

#[derive(Default)]
struct Viewer {
    tab_rects: Vec<(String, Rect)>,
    /// A dock no tab may enter.
    refused_dock: Option<&'static str>,
    /// The docks of the last drop asked about: `(from, to)`.
    asked: RefCell<Option<(Option<String>, Option<String>)>>,
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

    fn dock_drop_allowed(&self, _tab: &String, drop: &DockDrop<'_>) -> DropVerdict {
        *self.asked.borrow_mut() = Some((drop.from.map(str::to_owned), drop.to.map(str::to_owned)));
        if drop.to.is_some() && drop.to == self.refused_dock {
            DropVerdict::Refuse(Some("Not here".into()))
        } else {
            DropVerdict::Allow
        }
    }
}

/// Paints as the style says, and records each drop preview: its rect, where the tab would go,
/// and whether it is refused.
#[derive(Default)]
struct Previews(Mutex<Vec<(Rect, String, bool)>>);

/// `destination`, as text.
fn describe(destination: &TabDestination) -> String {
    match destination {
        TabDestination::Window(_) => "window".to_owned(),
        TabDestination::Node(path, insert) => {
            let insert = match insert {
                TabInsert::Split(split) => format!("split {split:?}"),
                TabInsert::Insert(index) => format!("insert {}", index.0),
                TabInsert::Append => "append".to_owned(),
            };
            format!("{path:?} {insert}")
        }
        TabDestination::EmptySurface(surface) => format!("empty {surface:?}"),
    }
}

impl DockPainter for Previews {
    fn drop_preview(&self, painter: &egui::Painter, drop: &DropPaint<'_>) {
        StylePainter.drop_preview(painter, drop);
        self.0.lock().unwrap().push((
            drop.rect,
            describe(drop.destination),
            drop.refusal.is_some(),
        ));
    }
}

struct App {
    space: DockSpace<String>,
    viewer: Viewer,
    /// The docks in the order they are drawn.
    order: Vec<&'static str>,
    previews: &'static Previews,
    /// The no-drop zones declared every frame.
    no_drop: Vec<Rect>,
}

/// Where each dock is drawn on the 900 × 400 screen; below 300 points is outside every dock.
fn dock_rect(name: &str) -> Rect {
    match name {
        "left" => Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 300.0)),
        "centre" => Rect::from_min_max(pos2(200.0, 0.0), pos2(700.0, 300.0)),
        "right" => Rect::from_min_max(pos2(700.0, 0.0), pos2(900.0, 300.0)),
        _ => unreachable!(),
    }
}

/// `Files` and `Search` on the left, `Editor` and `Preview` in the centre, nothing on the right.
fn space(left: DockRules, right: DockRules) -> DockSpace<String> {
    let mut space = DockSpace::new();
    space.add_dock("left", left, vec!["Files".to_owned(), "Search".to_owned()]);
    space.add_dock(
        "centre",
        DockRules::default(),
        vec!["Editor".to_owned(), "Preview".to_owned()],
    );
    space.add_dock("right", right, vec![]);
    space
}

fn harness_with(space: DockSpace<String>, order: Vec<&'static str>) -> Harness<'static, App> {
    let app = App {
        space,
        viewer: Viewer::default(),
        order,
        previews: Box::leak(Box::default()),
        no_drop: Vec::new(),
    };
    let mut harness = Harness::builder().with_size([900.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            app.viewer.tab_rects.clear();
            app.space.no_drop_zones(app.no_drop.clone());
            for &name in &app.order.clone() {
                ui.scope_builder(UiBuilder::new().max_rect(dock_rect(name)), |ui| {
                    DockArea::dock(&mut app.space, name)
                        .painter(app.previews)
                        .show_inside(ui, &mut app.viewer);
                });
            }
        },
        app,
    );
    harness.run();
    harness
}

fn harness() -> Harness<'static, App> {
    harness_with(
        space(DockRules::default(), DockRules::default()),
        vec!["left", "centre", "right"],
    )
}

fn tab_rect(harness: &Harness<'_, App>, tab: &str) -> Rect {
    harness
        .state()
        .viewer
        .tab_rects
        .iter()
        .find(|(name, _)| name == tab)
        .map(|(_, rect)| *rect)
        .unwrap_or_else(|| panic!("{tab} is shown"))
}

/// The rect of the root leaf of the dock `name`.
fn dock_leaf(harness: &Harness<'_, App>, name: &str) -> Rect {
    harness.state().space.dock(name).unwrap()[NodeIndex::root()]
        .rect()
        .unwrap()
}

/// Drags `tab` to where `target` says, holds it there, then releases it.
fn drop_at(harness: &mut Harness<'_, App>, tab: &str, target: impl Fn(&Harness<'_, App>) -> Pos2) {
    let target = drag_over(harness, tab, target);
    harness.drop_at(target);
    harness.run_steps(5);
}

/// Drags `tab` to where `target` says and holds it there; returns where.
fn drag_over(
    harness: &mut Harness<'_, App>,
    tab: &str,
    target: impl Fn(&Harness<'_, App>) -> Pos2,
) -> Pos2 {
    let start = tab_rect(harness, tab).center();
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
    target
}

/// The leaves of the dock `name`, in the tree's order.
fn leaves(app: &App, name: &str) -> Vec<Vec<String>> {
    app.space
        .dock(name)
        .unwrap()
        .iter()
        .filter_map(|node| node.tabs().map(<[String]>::to_vec))
        .collect()
}

fn strings(tabs: &[&str]) -> Vec<String> {
    tabs.iter().map(|tab| (*tab).to_owned()).collect()
}

/// Below the centre leaf's tab bar, in its middle.
fn centre_middle(harness: &Harness<'_, App>) -> Pos2 {
    let leaf = dock_leaf(harness, "centre");
    pos2(leaf.center().x, leaf.center().y + 30.0)
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_middle_of_another_docks_leaf_adds_the_tab_to_it() {
    let mut harness = harness();
    drop_at(&mut harness, "Files", centre_middle);
    assert_eq!(leaves(harness.state(), "left"), [strings(&["Search"])]);
    assert_eq!(
        leaves(harness.state(), "centre"),
        [strings(&["Editor", "Preview", "Files"])]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_docks_may_be_drawn_in_any_order() {
    let mut harness = harness_with(
        space(DockRules::default(), DockRules::default()),
        vec!["right", "centre", "left"],
    );
    drop_at(&mut harness, "Files", centre_middle);
    assert_eq!(
        leaves(harness.state(), "centre"),
        [strings(&["Editor", "Preview", "Files"])]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn an_edge_band_of_another_docks_leaf_splits_it() {
    type Side = fn(Rect) -> Pos2;
    type Placed = fn(Rect, Rect) -> bool;
    let sides: [(&str, Side, Placed); 4] = [
        (
            "left",
            |leaf| pos2(leaf.left() + 10.0, leaf.center().y),
            |files, editor| files.right() <= editor.left() + 1.0,
        ),
        (
            "right",
            |leaf| pos2(leaf.right() - 10.0, leaf.center().y),
            |files, editor| files.left() >= editor.right() - 1.0,
        ),
        (
            "top",
            |leaf| pos2(leaf.center().x, leaf.top() + 32.0),
            |files, editor| files.bottom() <= editor.top() + 1.0,
        ),
        (
            "bottom",
            |leaf| pos2(leaf.center().x, leaf.bottom() - 10.0),
            |files, editor| files.top() >= editor.bottom() - 1.0,
        ),
    ];
    for (name, at, placed) in sides {
        let mut harness = harness();
        drop_at(&mut harness, "Files", |h| at(dock_leaf(h, "centre")));

        let tree = harness.state().space.dock("centre").unwrap();
        let rect_of = |tab: &str| {
            let (node, _) = tree.find_tab(&tab.to_owned()).unwrap();
            tree[node].rect().unwrap()
        };
        let (files, editor) = (rect_of("Files"), rect_of("Editor"));
        assert_ne!(files, editor, "{name}: Files has a leaf of its own");
        assert!(placed(files, editor), "{name}: {files:?} beside {editor:?}");
        assert_eq!(
            leaves(harness.state(), "left"),
            [strings(&["Search"])],
            "{name}: Files left the left dock"
        );
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_of_another_docks_bar_takes_the_tab_before_it() {
    let mut harness = harness();
    drop_at(&mut harness, "Files", |h| tab_rect(h, "Preview").center());
    assert_eq!(
        leaves(harness.state(), "centre"),
        [strings(&["Editor", "Files", "Preview"])]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_viewer_may_refuse_a_dock() {
    let mut harness = harness();
    harness.state_mut().viewer.refused_dock = Some("centre");
    drop_at(&mut harness, "Files", centre_middle);
    assert_eq!(
        leaves(harness.state(), "left"),
        [strings(&["Files", "Search"])]
    );
    assert_eq!(
        leaves(harness.state(), "centre"),
        [strings(&["Editor", "Preview"])]
    );

    // Another dock still takes it.
    drop_at(&mut harness, "Files", |_| dock_rect("right").center());
    assert_eq!(leaves(harness.state(), "right"), [strings(&["Files"])]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_viewer_is_told_the_docks() {
    let mut harness = harness();
    drop_at(&mut harness, "Files", centre_middle);
    assert_eq!(
        harness.state().viewer.asked.borrow().clone(),
        Some((Some("left".to_owned()), Some("centre".to_owned())))
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dock_that_refuses_splits_takes_the_tab_in_its_leaf() {
    let mut harness = harness_with(
        space(
            DockRules::default().splits(AllowedSplits::None),
            DockRules::default(),
        ),
        vec!["left", "centre", "right"],
    );
    // The band along the left leaf's bottom edge.
    drop_at(&mut harness, "Editor", |h| {
        let leaf = dock_leaf(h, "left");
        pos2(leaf.center().x, leaf.bottom() - 10.0)
    });
    assert_eq!(
        leaves(harness.state(), "left"),
        [strings(&["Files", "Search", "Editor"])]
    );
    assert_eq!(leaves(harness.state(), "centre"), [strings(&["Preview"])]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn an_empty_dock_kept_is_a_target() {
    let mut harness = harness();
    drop_at(&mut harness, "Files", |_| dock_rect("right").center());
    assert_eq!(leaves(harness.state(), "right"), [strings(&["Files"])]);

    // Emptied again, it stays, and takes a tab again.
    drop_at(&mut harness, "Files", centre_middle);
    assert!(harness.state().space.has_dock("right"));
    assert!(harness.state().space.dock("right").unwrap().is_empty());
    drop_at(&mut harness, "Search", |_| dock_rect("right").center());
    assert_eq!(leaves(harness.state(), "right"), [strings(&["Search"])]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dock_not_kept_goes_once_empty() {
    let mut harness = harness_with(
        space(
            DockRules::default().keep_when_empty(false),
            DockRules::default(),
        ),
        vec!["left", "centre", "right"],
    );
    drop_at(&mut harness, "Files", centre_middle);
    assert!(harness.state().space.has_dock("left"));
    drop_at(&mut harness, "Search", centre_middle);
    assert!(!harness.state().space.has_dock("left"));
    assert_eq!(
        leaves(harness.state(), "centre"),
        [strings(&["Editor", "Preview", "Files", "Search"])]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn outside_every_dock_the_tab_opens_in_a_window_and_comes_back() {
    let mut harness = harness();
    drop_at(&mut harness, "Files", |_| pos2(450.0, 350.0));
    let state = harness.state().space.state();
    let files = state.find_tab(&"Files".to_owned()).unwrap();
    assert!(state.get_window_state(files.surface).is_some(), "a window");
    assert_eq!(harness.state().space.dock_name(files.surface), None);

    // From the window into another dock.
    drop_at(&mut harness, "Files", |_| dock_rect("right").center());
    assert_eq!(leaves(harness.state(), "right"), [strings(&["Files"])]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dock_without_windows_keeps_its_tabs_docked() {
    let mut harness = harness_with(
        space(DockRules::default().windows(false), DockRules::default()),
        vec!["left", "centre", "right"],
    );
    drop_at(&mut harness, "Files", |_| pos2(450.0, 350.0));
    assert_eq!(
        leaves(harness.state(), "left"),
        [strings(&["Files", "Search"])]
    );
}

#[test]
fn the_space_names_its_docks() {
    let space = space(DockRules::default(), DockRules::default());
    assert_eq!(
        space.dock_names().collect::<Vec<_>>(),
        ["left", "centre", "right"]
    );
    let centre = space.dock_surface("centre").unwrap();
    assert_eq!(space.dock_name(centre), Some("centre"));
    assert_eq!(space.windows_dock(), Some("left"));
    assert_eq!(
        space.rules("right"),
        Some(DockRules::default()),
        "the rules given"
    );
}

#[test]
fn several_dock_states_become_one_space() {
    let mut left = DockState::new(strings(&["Files"]));
    left.add_window(strings(&["Floating"]));
    let centre = DockState::new(strings(&["Editor"]));
    let space = DockSpace::from_states([
        ("left".to_owned(), DockRules::default(), left),
        ("centre".to_owned(), DockRules::default(), centre),
    ]);
    let tabs = |name: &str| {
        space
            .dock(name)
            .unwrap()
            .iter()
            .filter_map(|node| node.tabs().map(<[String]>::to_vec))
            .collect::<Vec<_>>()
    };
    assert_eq!(tabs("left"), [strings(&["Files"])]);
    assert_eq!(tabs("centre"), [strings(&["Editor"])]);
    let floating = space.state().find_tab(&"Floating".to_owned()).unwrap();
    assert!(space.state().get_window_state(floating.surface).is_some());
}

#[test]
fn retaining_tabs_keeps_every_docks_surface() {
    let mut space = space(DockRules::default(), DockRules::default());
    let window = space.state_mut().add_window(strings(&["Floating"]));
    space.add_dock("bottom", DockRules::default(), strings(&["Console"]));
    let bottom = space.dock_surface("bottom").unwrap();
    assert!(
        bottom.0 > window.0,
        "the bottom dock comes after the window"
    );
    space.state_mut().retain_tabs(|tab| tab != "Floating");
    assert_eq!(space.dock_surface("bottom"), Some(bottom));
    assert_eq!(
        space
            .dock("bottom")
            .unwrap()
            .find_tab(&"Console".to_owned()),
        Some((NodeIndex::root(), egui_stack::TabIndex(0)))
    );
}

#[test]
fn removing_a_dock_gives_its_tree() {
    let mut space = space(DockRules::default(), DockRules::default());
    let tree: Tree<String> = space.remove_dock("left").unwrap();
    assert_eq!(
        tree.find_tab(&"Files".to_owned()).map(|(_, tab)| tab.0),
        Some(0)
    );
    assert!(!space.has_dock("left"));
    assert_eq!(space.windows_dock(), Some("centre"));
    // Its surface is free for the next dock.
    space.add_dock("left", DockRules::default(), strings(&["Files"]));
    assert!(space.has_dock("left"));
}

#[cfg(feature = "serde")]
#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_space_saves_and_loads_as_one_value() {
    let left = DockRules::default()
        .splits(AllowedSplits::TopBottomOnly)
        .keep_when_empty(false);
    let mut space = space(left, DockRules::default().windows(false));
    space.state_mut().add_window(strings(&["Floating"]));
    space.set_windows_dock("centre");
    // Shown once, so that every leaf has a rect: a leaf never shown has none, which JSON
    // cannot hold.
    let space = harness_with(space, vec!["left", "centre", "right"])
        .state()
        .space
        .clone();
    let json = serde_json::to_string(&space).unwrap();
    let loaded: DockSpace<String> = serde_json::from_str(&json).unwrap();

    assert_eq!(
        loaded.dock_names().collect::<Vec<_>>(),
        ["left", "centre", "right"]
    );
    assert_eq!(loaded.rules("left"), Some(left));
    assert_eq!(
        loaded.rules("right"),
        Some(DockRules::default().windows(false))
    );
    assert_eq!(loaded.windows_dock(), Some("centre"));
    for name in ["left", "centre", "right"] {
        assert_eq!(loaded.dock_surface(name), space.dock_surface(name));
    }
    let tabs = |space: &DockSpace<String>| {
        space
            .state()
            .iter_all_tabs()
            .map(|(path, tab)| (path, tab.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(tabs(&loaded), tabs(&space));
    assert_eq!(serde_json::to_string(&loaded).unwrap(), json);
}

/// The band along the centre leaf's right edge.
fn centre_right_band(harness: &Harness<'_, App>) -> Pos2 {
    let leaf = dock_leaf(harness, "centre");
    pos2(leaf.right() - 10.0, leaf.center().y)
}

fn last_preview(harness: &Harness<'_, App>) -> (Rect, String, bool) {
    harness
        .state()
        .previews
        .0
        .lock()
        .unwrap()
        .last()
        .cloned()
        .expect("a preview")
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_preview_in_another_dock_is_the_region_the_tab_would_take() {
    let mut split = harness();
    drag_over(&mut split, "Files", centre_right_band);
    let leaf = dock_leaf(&split, "centre");
    let path = NodePath {
        surface: split.state().space.dock_surface("centre").unwrap(),
        node: NodeIndex::root(),
    };
    let (rect, destination, is_refused) = last_preview(&split);
    assert_eq!(
        rect,
        leaf.intersect(Rect::everything_right_of(leaf.center().x))
    );
    assert_eq!(
        destination,
        describe(&TabDestination::Node(path, TabInsert::Split(Split::Right)))
    );
    assert!(!is_refused);
    check("space_drop_split", &split);

    // Refused by the viewer: the same region, shown as refused.
    let mut refused = harness();
    refused.state_mut().viewer.refused_dock = Some("centre");
    drag_over(&mut refused, "Files", centre_right_band);
    let (refused_rect, _, is_refused) = last_preview(&refused);
    assert_eq!(refused_rect, rect);
    assert!(is_refused);
    check("space_drop_refused", &refused);
}

fn shape_lines(out: &mut String, depth: usize, shape: &Shape) {
    let indent = "  ".repeat(depth);
    match shape {
        Shape::Vec(shapes) => {
            let _ = writeln!(out, "{indent}Vec");
            for shape in shapes {
                shape_lines(out, depth + 1, shape);
            }
        }
        // A galley's glyphs depend on the font atlas: its text, place and colours are enough.
        Shape::Text(text) => {
            let _ = writeln!(
                out,
                "{indent}Text {:?} at {:?} size {:?} fallback {:?} override {:?} opacity {:?}",
                text.galley.text(),
                text.pos,
                text.galley.size(),
                text.fallback_color,
                text.override_text_color,
                text.opacity_factor,
            );
        }
        shape => {
            let _ = writeln!(out, "{indent}{shape:?}");
        }
    }
}

/// Compares the frame's shapes with `tests/painting/<scene>.txt`; `UPDATE_SNAPSHOTS=1` writes
/// it again after an intended change.
fn check(scene: &str, harness: &Harness<'_, App>) {
    let mut actual = String::new();
    for ClippedShape {
        clip_rect, shape, ..
    } in &harness.output().shapes
    {
        let _ = writeln!(actual, "clip {clip_rect:?}");
        shape_lines(&mut actual, 1, shape);
    }
    let path = format!("{}/tests/painting/{scene}.txt", env!("CARGO_MANIFEST_DIR"));
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{path}: {e}; run with UPDATE_SNAPSHOTS=1 to write it"));
    // Line by line: a checkout may have turned the files' line ends into CRLF.
    if !actual.lines().eq(expected.lines()) {
        let new = format!("{path}.new");
        std::fs::write(&new, &actual).unwrap();
        panic!("{scene}: the shapes differ (see {new})");
    }
}

/// The tabs of the floating window `surface`.
fn window_tabs(app: &App, surface: SurfaceIndex) -> Vec<String> {
    app.space.state()[surface]
        .iter()
        .filter_map(|node| node.tabs().map(<[String]>::to_vec))
        .flatten()
        .collect()
}

/// The space, with a floating window holding `Notes` at `rect`; the window is drawn by the
/// first dock, before the docks under it.
fn harness_with_window(rect: Rect) -> (Harness<'static, App>, SurfaceIndex) {
    let mut space = space(DockRules::default(), DockRules::default());
    let window = space.state_mut().add_window(vec!["Notes".to_owned()]);
    let mut harness = harness_with(space, vec!["left", "centre", "right"]);
    harness
        .state_mut()
        .space
        .state_mut()
        .get_window_state_mut(window)
        .unwrap()
        .set_position(rect.min)
        .set_size(rect.size());
    harness.run_steps(3);
    (harness, window)
}

/// The rect of the floating window's root leaf.
fn window_leaf(harness: &Harness<'_, App>, window: SurfaceIndex) -> Rect {
    harness.state().space.state()[window][NodeIndex::root()]
        .rect()
        .unwrap()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_dropped_on_a_floating_window_over_a_dock_lands_in_the_window() {
    let (mut harness, window) =
        harness_with_window(Rect::from_min_size(pos2(350.0, 60.0), vec2(200.0, 160.0)));
    drop_at(&mut harness, "Search", |harness| {
        let leaf = window_leaf(harness, window);
        pos2(leaf.center().x, leaf.center().y + 20.0)
    });
    assert_eq!(
        window_tabs(harness.state(), window),
        strings(&["Notes", "Search"])
    );
    assert_eq!(
        leaves(harness.state(), "centre"),
        [strings(&["Editor", "Preview"])]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_dropped_on_the_tab_bar_of_a_window_over_two_docks_lands_in_the_window() {
    // The window straddles the edge between the centre and the right dock, at x = 700.
    let (mut harness, window) =
        harness_with_window(Rect::from_min_size(pos2(600.0, 60.0), vec2(250.0, 160.0)));
    let leaf = window_leaf(&harness, window);
    assert!(
        leaf.x_range().contains(700.0) && leaf.right() > 760.0,
        "{leaf:?}"
    );
    drop_at(&mut harness, "Search", |harness| {
        pos2(750.0, window_leaf(harness, window).top() + 10.0)
    });
    assert_eq!(
        window_tabs(harness.state(), window),
        strings(&["Notes", "Search"])
    );
    assert!(harness.state().space.dock("right").unwrap().is_empty());
}

/// Holds `tab` over `target`, then says whether a drop preview showed there, and releases it.
fn previewed_then_dropped(
    harness: &mut Harness<'_, App>,
    tab: &str,
    target: impl Fn(&Harness<'_, App>) -> Pos2,
) -> bool {
    let target = drag_over(harness, tab, target);
    harness.state().previews.0.lock().unwrap().clear();
    harness.hover_at(target);
    harness.step();
    harness.step();
    let previewed = !harness.state().previews.0.lock().unwrap().is_empty();
    harness.drop_at(target);
    harness.run_steps(5);
    previewed
}

fn in_a_window(app: &App, tab: &str) -> bool {
    let state = app.space.state();
    let found = state.find_tab(&tab.to_owned()).unwrap();
    state.get_window_state(found.surface).is_some()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_released_in_a_no_drop_zone_goes_back_and_does_not_float() {
    let mut harness = harness();
    // The status bar under the docks.
    harness.state_mut().no_drop = vec![Rect::from_min_max(pos2(0.0, 320.0), pos2(900.0, 400.0))];
    let previewed = previewed_then_dropped(&mut harness, "Files", |_| pos2(450.0, 350.0));
    assert!(!previewed, "no preview in the zone");
    assert!(!in_a_window(harness.state(), "Files"));
    assert_eq!(
        leaves(harness.state(), "left"),
        [strings(&["Files", "Search"])]
    );

    // Out of the zone, the space outside every dock still makes a window.
    let previewed = previewed_then_dropped(&mut harness, "Files", |_| pos2(450.0, 310.0));
    assert!(previewed);
    assert!(in_a_window(harness.state(), "Files"));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_no_drop_zone_over_a_dock_wins_and_is_forgotten_once_not_declared() {
    let mut harness = harness();
    harness.state_mut().no_drop = vec![dock_rect("centre")];
    let previewed = previewed_then_dropped(&mut harness, "Files", centre_middle);
    assert!(!previewed);
    assert_eq!(
        leaves(harness.state(), "centre"),
        [strings(&["Editor", "Preview"])]
    );

    // Declared no more, the zone is gone.
    harness.state_mut().no_drop.clear();
    drop_at(&mut harness, "Files", centre_middle);
    assert_eq!(
        leaves(harness.state(), "centre"),
        [strings(&["Editor", "Preview", "Files"])]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dock_that_refuses_drops_is_no_target_but_its_tabs_leave_it() {
    let mut harness = harness_with(
        space(
            DockRules::default().accept_drops(false),
            DockRules::default(),
        ),
        vec!["left", "centre", "right"],
    );
    let previewed = previewed_then_dropped(&mut harness, "Editor", |h| {
        let leaf = dock_leaf(h, "left");
        pos2(leaf.center().x, leaf.center().y + 30.0)
    });
    assert!(!previewed, "no preview over the dock");
    assert!(!in_a_window(harness.state(), "Editor"), "nor a window");
    assert_eq!(
        leaves(harness.state(), "centre"),
        [strings(&["Editor", "Preview"])]
    );

    // Its own tabs still go out.
    drop_at(&mut harness, "Files", centre_middle);
    assert_eq!(leaves(harness.state(), "left"), [strings(&["Search"])]);
    assert_eq!(
        leaves(harness.state(), "centre"),
        [strings(&["Editor", "Preview", "Files"])]
    );
}
