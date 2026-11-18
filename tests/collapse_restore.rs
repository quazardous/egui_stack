//! Collapsible zones (`Fold::Collapse`): a zone whose docks are empty collapses to nothing
//! (`DockSpace::collapse_empty`), its neighbours taking its room, and a tab dragged onto its
//! restore band, at its former boundary, brings it back at its remembered size
//! (`DockSpace::take_unfolds`). A cancelled drag, a release elsewhere or a veto leave the layout
//! as it was.

use egui::{Key, Pos2, Rect, Ui, WidgetText, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::layout::{
    Content, Dir, Fold, Layout, LayoutArea, Shown, Size, Zone, ZoneName, ZoneViewer,
};
use egui_stack::{DockArea, DockDrop, DockRules, DockSpace, TabViewer, tab_viewer::DropVerdict};

struct Tabs {
    /// A dock the viewer refuses every tab in.
    refuse: Option<&'static str>,
}

impl TabViewer for Tabs {
    type Tab = String;

    fn id(&mut self, tab: &mut String) -> egui::Id {
        egui::Id::unique(tab.as_str())
    }

    fn title(&mut self, tab: &mut String) -> WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut String) {
        ui.label(format!("Body of {tab}"));
    }

    fn dock_drop_allowed(&self, _tab: &String, drop: &DockDrop<'_>) -> DropVerdict {
        if drop.to.is_some() && drop.to == self.refuse {
            DropVerdict::Refuse(None)
        } else {
            DropVerdict::Allow
        }
    }
}

struct Viewer<'a> {
    space: &'a mut DockSpace<String>,
    refuse: Option<&'static str>,
}

impl ZoneViewer for Viewer<'_> {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        match zone.0.as_str() {
            "viewport" => {
                ui.label("Viewport");
            }
            dock => {
                let mut tabs = Tabs {
                    refuse: self.refuse,
                };
                DockArea::dock(self.space, dock).show_inside(ui, &mut tabs);
            }
        }
    }
}

struct App {
    layout: Layout,
    space: DockSpace<String>,
    unfolded: Vec<ZoneName>,
    refuse: Option<&'static str>,
}

const SCREEN: Rect = Rect::from_min_max(Pos2::new(8.0, 8.0), Pos2::new(892.0, 392.0));

fn side(name: &str) -> Zone {
    Zone::leaf(name, Content::Dock(name.into())).with(|c| {
        c.default = Some(Size::Points(200.0));
        c.fold = Fold::Collapse;
    })
}

/// The reporter's case: a viewport between two side docks, empty, over a bottom dock holding
/// the tabs. `sides` are the zones left of the viewport.
fn layout(sides: &[&str]) -> Layout {
    let mut main: Vec<Zone> = sides.iter().map(|name| side(name)).collect();
    main.push(Zone::leaf("viewport", Content::App).with(|c| c.grow = true));
    main.push(side("right"));
    Layout::new(Zone::split(
        "root",
        Dir::Vertical,
        vec![
            Zone::split("main", Dir::Horizontal, main).with(|c| c.grow = true),
            Zone::leaf("bottom", Content::Dock("bottom".into()))
                .with(|c| c.default = Some(Size::Points(120.0))),
        ],
    ))
}

fn harness_with(layout: Layout) -> Harness<'static, App> {
    let mut space = DockSpace::new();
    space.add_dock(
        "bottom",
        DockRules::default(),
        vec!["Console".to_owned(), "Log".to_owned()],
    );
    layout.root.visit(&mut |zone| {
        if let egui_stack::layout::ZoneKind::Leaf {
            content: Content::Dock(name),
        } = &zone.kind
            && !space.has_dock(name)
        {
            space.add_dock(name.clone(), DockRules::default(), vec![]);
        }
    });
    let mut harness = Harness::builder().with_size([900.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            app.space.collapse_empty(&mut app.layout);
            let laid = LayoutArea::new(&mut app.layout).show_inside(
                ui,
                &mut Viewer {
                    space: &mut app.space,
                    refuse: app.refuse,
                },
            );
            app.space.bind(&laid);
            for zone in app.space.take_unfolds() {
                app.layout.set_folded(&zone.0, false);
                app.unfolded.push(zone);
            }
        },
        App {
            layout,
            space,
            unfolded: Vec::new(),
            refuse: None,
        },
    );
    harness.run();
    harness
}

fn harness() -> Harness<'static, App> {
    harness_with(layout(&["left"]))
}

/// A point on the band of the zone `name`, `depth` points in from its boundary.
fn band(harness: &Harness<'_, App>, name: &str, depth: f32) -> Pos2 {
    let laid = harness.state().layout.solve(SCREEN, &Default::default());
    let restore = laid.get(name).unwrap().restore.unwrap();
    let line = restore.line.center();
    if restore.towards_end {
        line + vec2(depth, 0.0)
    } else {
        line - vec2(depth, 0.0)
    }
}

/// Drags `tab` to `target` and holds it there.
fn drag_to(harness: &mut Harness<'_, App>, tab: &str, target: Pos2) {
    let start = harness.get_by_label(tab).rect().center();
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=5 {
        harness.hover_at(start - vec2(0.0, 10.0 * step as f32));
        harness.step();
    }
    for _ in 0..5 {
        harness.hover_at(target);
        harness.step();
    }
}

/// Drags `tab` to `target`, holds it there, then releases it.
fn drop_at(harness: &mut Harness<'_, App>, tab: &str, target: Pos2) {
    drag_to(harness, tab, target);
    harness.drop_at(target);
    harness.run_steps(5);
}

fn dock_of(app: &App, tab: &str) -> Option<String> {
    let path = app.space.state().find_tab(&tab.to_owned()).unwrap();
    app.space.dock_name(path.surface).map(str::to_owned)
}

fn shown(app: &App, name: &str) -> Shown {
    app.layout
        .solve(SCREEN, &Default::default())
        .get(name)
        .unwrap()
        .shown
}

fn width(app: &App, name: &str) -> f32 {
    app.layout
        .solve(SCREEN, &Default::default())
        .rect(name)
        .unwrap()
        .width()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn empty_side_docks_collapse_and_the_viewport_takes_all_the_room() {
    let harness = harness();
    let app = harness.state();
    assert_eq!(shown(app, "left"), Shown::Collapsed);
    assert_eq!(shown(app, "right"), Shown::Collapsed);
    assert_eq!(width(app, "viewport"), SCREEN.width());
    // Still in the tree, with their constraints.
    assert!(app.layout.zone("left").unwrap().state.folded);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn one_drag_onto_the_band_brings_the_dock_back() {
    let mut harness = harness();
    let target = band(&harness, "left", 4.0);
    drop_at(&mut harness, "Log", target);
    let app = harness.state();
    assert_eq!(dock_of(app, "Log").as_deref(), Some("left"));
    assert_eq!(app.unfolded, [ZoneName::from("left")]);
    assert_eq!(shown(app, "left"), Shown::Open);
    assert_eq!(width(app, "left"), 200.0);
    // The other side stays collapsed.
    assert_eq!(shown(app, "right"), Shown::Collapsed);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_right_band_lies_inside_the_right_edge() {
    let mut harness = harness();
    let target = band(&harness, "right", 4.0);
    assert!(target.x < SCREEN.max.x && target.x > SCREEN.max.x - 8.0);
    drop_at(&mut harness, "Log", target);
    let app = harness.state();
    assert_eq!(dock_of(app, "Log").as_deref(), Some("right"));
    assert_eq!(width(app, "right"), 200.0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn emptied_again_the_dock_collapses_again() {
    let mut harness = harness();
    let target = band(&harness, "left", 4.0);
    drop_at(&mut harness, "Log", target);
    assert_eq!(shown(harness.state(), "left"), Shown::Open);
    let bottom = harness.get_by_label("Console").rect().center();
    drop_at(&mut harness, "Log", bottom);
    assert_eq!(dock_of(harness.state(), "Log").as_deref(), Some("bottom"));
    assert_eq!(shown(harness.state(), "left"), Shown::Collapsed);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_release_off_the_band_leaves_the_layout_unchanged() {
    let mut harness = harness();
    let before = harness.state().layout.clone();
    // Past the band, over the viewport, which takes no drops.
    let target = band(&harness, "left", 40.0);
    drop_at(&mut harness, "Log", target);
    let app = harness.state();
    assert_eq!(dock_of(app, "Log").as_deref(), Some("bottom"));
    assert!(app.unfolded.is_empty());
    assert_eq!(app.layout, before);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_cancelled_drag_leaves_the_layout_unchanged() {
    let mut harness = harness();
    let before = harness.state().layout.clone();
    let target = band(&harness, "left", 4.0);
    drag_to(&mut harness, "Log", target);
    harness.key_press(Key::Escape);
    harness.step();
    harness.drop_at(target);
    harness.run_steps(5);
    let app = harness.state();
    assert_eq!(dock_of(app, "Log").as_deref(), Some("bottom"));
    assert!(app.unfolded.is_empty());
    assert_eq!(app.layout, before);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_veto_leaves_the_layout_unchanged() {
    let mut harness = harness();
    harness.state_mut().refuse = Some("left");
    harness.run();
    let before = harness.state().layout.clone();
    let target = band(&harness, "left", 4.0);
    drop_at(&mut harness, "Log", target);
    let app = harness.state();
    assert_eq!(dock_of(app, "Log").as_deref(), Some("bottom"));
    assert!(app.unfolded.is_empty());
    assert_eq!(app.layout, before);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn bands_on_one_boundary_stack_in_tree_order() {
    let mut harness = harness_with(layout(&["tools", "left"]));
    // Both collapsed at the main split's left edge: "tools" first, then "left", 8 points each.
    let tools = band(&harness, "tools", 4.0);
    let left = band(&harness, "left", 12.0);
    assert_eq!(tools.x, SCREEN.min.x + 4.0);
    assert_eq!(left.x, SCREEN.min.x + 12.0);
    drop_at(&mut harness, "Log", left);
    let app = harness.state();
    assert_eq!(dock_of(app, "Log").as_deref(), Some("left"));
    assert_eq!(shown(app, "left"), Shown::Open);
    assert_eq!(shown(app, "tools"), Shown::Collapsed);

    let mut harness = harness_with(layout(&["tools", "left"]));
    drop_at(&mut harness, "Log", tools);
    let app = harness.state();
    assert_eq!(dock_of(app, "Log").as_deref(), Some("tools"));
    assert_eq!(shown(app, "left"), Shown::Collapsed);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn with_its_neighbour_gone_the_band_lies_on_the_outer_edge() {
    // A loaded layout had a toolbar before "left"; the code no longer has it.
    let mut loaded = layout(&["toolbar", "left"]);
    loaded.constrain("toolbar", |c| {
        c.fold = Fold::Never;
        c.default = Some(Size::Points(40.0));
    });
    if let Some(zone) = loaded.zone_mut("toolbar") {
        zone.kind = egui_stack::layout::ZoneKind::Leaf {
            content: Content::App,
        };
    }
    let repair = loaded.reconcile(&layout(&["left"]));
    assert_eq!(repair.removed, [ZoneName::from("toolbar")]);
    let mut harness = harness_with(loaded);
    let target = band(&harness, "left", 4.0);
    assert_eq!(target.x, SCREEN.min.x + 4.0);
    drop_at(&mut harness, "Log", target);
    assert_eq!(dock_of(harness.state(), "Log").as_deref(), Some("left"));
    assert_eq!(shown(harness.state(), "left"), Shown::Open);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_hidden_zone_shows_no_band() {
    let mut harness = harness();
    harness
        .state_mut()
        .layout
        .zone_mut("left")
        .unwrap()
        .state
        .hidden = true;
    harness.run();
    let laid = harness.state().layout.solve(SCREEN, &Default::default());
    assert_eq!(laid.get("left").unwrap().shown, Shown::Hidden);
    assert!(laid.get("left").unwrap().restore.is_none());
    let target = pos_left_edge();
    drop_at(&mut harness, "Log", target);
    assert_eq!(dock_of(harness.state(), "Log").as_deref(), Some("bottom"));
}

fn pos_left_edge() -> Pos2 {
    Pos2::new(SCREEN.min.x + 4.0, SCREEN.center().y - 60.0)
}
