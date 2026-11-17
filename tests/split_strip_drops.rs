//! A folded split zone's strip taking drops (`Constraints::strip_drops` on a split): a tab
//! released there joins one of the docks the split holds, the last active one, else the first
//! that accepts drops, and the zone is reported to unfold (`DockSpace::take_unfolds`). Off by
//! default: the strip refuses drops as before.

use egui::{Pos2, Rect, Ui, WidgetText, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::layout::{
    Content, Dir, Drops, Fold, Layout, LayoutArea, Size, Zone, ZoneName, ZoneViewer,
};
use egui_stack::{DockArea, DockRules, DockSpace, NodeIndex, NodePath, TabViewer};

struct Tabs;

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
}

struct Viewer<'a> {
    space: &'a mut DockSpace<String>,
}

impl ZoneViewer for Viewer<'_> {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        DockArea::dock(self.space, &zone.0).show_inside(ui, &mut Tabs);
    }
}

struct App {
    layout: Layout,
    space: DockSpace<String>,
    unfolded: Vec<ZoneName>,
}

const SCREEN: Rect = Rect::from_min_max(Pos2::new(8.0, 8.0), Pos2::new(892.0, 392.0));

/// A side split, `top` over `bottom`, folded to its strip; a centre dock with two tabs.
fn harness(strip_drops: bool, bottom_rules: DockRules) -> Harness<'static, App> {
    let mut space = DockSpace::new();
    space.add_dock("top", DockRules::default(), vec!["Outline".to_owned()]);
    space.add_dock("bottom", bottom_rules, vec!["Search".to_owned()]);
    space.add_dock(
        "centre",
        DockRules::default(),
        vec!["Editor".to_owned(), "Preview".to_owned()],
    );
    let mut layout = Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::split(
                "side",
                Dir::Vertical,
                vec![
                    Zone::leaf("top", Content::Dock("top".into())),
                    Zone::leaf("bottom", Content::Dock("bottom".into())),
                ],
            )
            .with(|c| {
                c.default = Some(Size::Points(250.0));
                c.fold = Fold::ToStrip;
                c.drops = Drops::Accept;
                c.strip_drops = strip_drops;
            }),
            Zone::leaf("centre", Content::Dock("centre".into())).with(|c| c.grow = true),
        ],
    ));
    layout.set_folded("side", true);
    let mut harness = Harness::builder().with_size([900.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            let laid = LayoutArea::new(&mut app.layout).show_inside(
                ui,
                &mut Viewer {
                    space: &mut app.space,
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
        },
    );
    harness.run();
    harness
}

fn strip(harness: &Harness<'_, App>) -> Pos2 {
    let laid = harness.state().layout.solve(SCREEN, &Default::default());
    laid.get("side").unwrap().strip.unwrap().center() + vec2(0.0, 60.0)
}

/// Drags `tab` to `target`, holds it there, then releases it.
fn drop_at(harness: &mut Harness<'_, App>, tab: &str, target: Pos2) {
    let start = harness.get_by_label(tab).rect().center();
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
}

fn dock_of(app: &App, tab: &str) -> Option<String> {
    let path = app.space.state().find_tab(&tab.to_owned()).unwrap();
    app.space.dock_name(path.surface).map(str::to_owned)
}

fn folded(app: &App) -> bool {
    app.layout.zone("side").unwrap().state.folded
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_folded_split_strip_refuses_drops_by_default() {
    let mut harness = harness(false, DockRules::default());
    let target = strip(&harness);
    drop_at(&mut harness, "Preview", target);
    // The strip takes no drops and is a no-drop zone: the tab stays where it was, in no
    // floating window.
    assert_eq!(
        dock_of(harness.state(), "Preview").as_deref(),
        Some("centre")
    );
    assert!(harness.state().unfolded.is_empty());
    assert!(folded(harness.state()));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_folded_split_strip_whose_docks_all_refuse_drops_is_a_no_drop_zone() {
    let mut harness = harness(true, DockRules::default().accept_drops(false));
    harness
        .state_mut()
        .space
        .set_rules("top", DockRules::default().accept_drops(false));
    harness.run();
    let target = strip(&harness);
    drop_at(&mut harness, "Preview", target);
    assert_eq!(
        dock_of(harness.state(), "Preview").as_deref(),
        Some("centre")
    );
    assert!(harness.state().unfolded.is_empty());
    assert!(folded(harness.state()));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_dropped_on_the_strip_joins_the_first_dock_and_the_zone_unfolds() {
    let mut harness = harness(true, DockRules::default());
    let target = strip(&harness);
    drop_at(&mut harness, "Preview", target);
    assert_eq!(dock_of(harness.state(), "Preview").as_deref(), Some("top"));
    let top = harness.state().space.dock("top").unwrap();
    assert_eq!(
        top[NodeIndex::root()].get_leaf().unwrap().tabs,
        ["Outline", "Preview"]
    );
    assert_eq!(harness.state().unfolded, [ZoneName::from("side")]);
    assert!(!folded(harness.state()));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_last_active_dock_of_the_split_takes_the_tab() {
    let mut harness = harness(true, DockRules::default());
    let bottom = harness.state().space.dock_surface("bottom").unwrap();
    harness
        .state_mut()
        .space
        .state_mut()
        .set_focused_node_and_surface(NodePath::new(bottom, NodeIndex::root()));
    harness.run();
    let target = strip(&harness);
    drop_at(&mut harness, "Preview", target);
    assert_eq!(
        dock_of(harness.state(), "Preview").as_deref(),
        Some("bottom")
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dock_refusing_drops_is_passed_over() {
    let mut harness = harness(true, DockRules::default());
    harness
        .state_mut()
        .space
        .set_rules("top", DockRules::default().accept_drops(false));
    harness.run();
    let target = strip(&harness);
    drop_at(&mut harness, "Preview", target);
    assert_eq!(
        dock_of(harness.state(), "Preview").as_deref(),
        Some("bottom")
    );
    assert_eq!(harness.state().unfolded, [ZoneName::from("side")]);
}
