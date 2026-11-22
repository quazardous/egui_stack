//! A tab dropped on a folded zone's strip (`Constraints::strip_drops`) reports the zone to
//! unfold (`DockSpace::take_unfolds`), whether the zone holds one dock or splits into several;
//! egui_stack unfolds nothing itself. With the `bus` feature, the drop's `stack.dock.drop` ask
//! and its `TabMoved` change name the zone as well as the dock.

use egui::{Pos2, Rect, Ui, WidgetText, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::layout::{
    Content, Dir, Drops, Fold, Layout, LayoutArea, Size, Zone, ZoneName, ZoneViewer,
};
use egui_stack::{DockArea, DockRules, DockSpace, TabViewer};

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
    /// What `take_unfolds` reported, frame after frame; nothing is unfolded.
    unfolds: Vec<ZoneName>,
}

const SCREEN: Rect = Rect::from_min_max(Pos2::new(8.0, 8.0), Pos2::new(892.0, 392.0));

fn folded(c: &mut egui_stack::layout::Constraints) {
    c.default = Some(Size::Points(200.0));
    c.fold = Fold::ToStrip;
    c.drops = Drops::Accept;
    c.strip_drops = true;
}

/// A folded single-dock zone `left`, a folded split zone `side` (`top` over `bottom`), and a
/// centre dock with two tabs.
fn harness(space: DockSpace<String>) -> Harness<'static, App> {
    let mut space = space;
    space.add_dock("left", DockRules::default(), vec!["Files".to_owned()]);
    space.add_dock("top", DockRules::default(), vec!["Outline".to_owned()]);
    space.add_dock("bottom", DockRules::default(), vec!["Search".to_owned()]);
    space.add_dock(
        "centre",
        DockRules::default(),
        vec!["Editor".to_owned(), "Preview".to_owned()],
    );
    let mut layout = Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::leaf("left", Content::Dock("left".into())).with(folded),
            Zone::split(
                "side",
                Dir::Vertical,
                vec![
                    Zone::leaf("top", Content::Dock("top".into())),
                    Zone::leaf("bottom", Content::Dock("bottom".into())),
                ],
            )
            .with(folded),
            Zone::leaf("centre", Content::Dock("centre".into())).with(|c| c.grow = true),
        ],
    ));
    layout.set_folded("left", true);
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
            app.unfolds.extend(app.space.take_unfolds());
        },
        App {
            layout,
            space,
            unfolds: Vec::new(),
        },
    );
    harness.run();
    harness
}

fn strip(harness: &Harness<'_, App>, zone: &str) -> Pos2 {
    let laid = harness.state().layout.solve(SCREEN, &Default::default());
    laid.get(zone).unwrap().strip.unwrap().center() + vec2(0.0, 60.0)
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

fn is_folded(app: &App, zone: &str) -> bool {
    app.layout.zone(zone).unwrap().state.folded
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_drop_on_a_folded_single_dock_strip_is_reported_to_unfold() {
    let mut harness = harness(DockSpace::new());
    let target = strip(&harness, "left");
    drop_at(&mut harness, "Preview", target);
    assert_eq!(dock_of(harness.state(), "Preview").as_deref(), Some("left"));
    assert_eq!(harness.state().unfolds, [ZoneName::from("left")]);
    // Reported only: the application unfolds it, or not.
    assert!(is_folded(harness.state(), "left"));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_drop_on_a_folded_split_strip_is_reported_to_unfold() {
    let mut harness = harness(DockSpace::new());
    let target = strip(&harness, "side");
    drop_at(&mut harness, "Preview", target);
    assert_eq!(dock_of(harness.state(), "Preview").as_deref(), Some("top"));
    assert_eq!(harness.state().unfolds, [ZoneName::from("side")]);
    assert!(is_folded(harness.state(), "side"));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_drop_in_an_open_dock_reports_nothing_to_unfold() {
    let mut harness = harness(DockSpace::new());
    let target = harness.get_by_label("Editor").rect().center();
    drop_at(&mut harness, "Preview", target);
    assert!(harness.state().unfolds.is_empty());
}

#[cfg(feature = "bus")]
mod bus {
    use std::sync::{Arc, Mutex};

    use egui_stack::bus::bus_contract::{Bus, ListenOpts};
    use egui_stack::bus::{self, Change};

    use super::*;
    use bus_kernel::Kernel;

    /// The zones `stack.dock.drop` was asked for, and those `TabMoved` named.
    #[derive(Default)]
    struct Seen {
        asked: Vec<(Option<String>, Option<String>)>,
        moved: Vec<(Option<String>, Option<String>)>,
    }

    fn with_bus() -> (Harness<'static, App>, Arc<Mutex<Seen>>) {
        let bus = Arc::new(Kernel::new());
        let space = DockSpace::new().with_bus(bus.clone(), |tab: &String| tab.clone());
        let seen = Arc::new(Mutex::new(Seen::default()));
        let asked = seen.clone();
        let listen: &dyn Bus = &*bus;
        listen
            .listen::<bus::DockDrop>("test.drop", ListenOpts::new(), move |ask, _| {
                asked
                    .lock()
                    .unwrap()
                    .asked
                    .push((ask.to.clone(), ask.zone.clone()));
                Ok(())
            })
            .unwrap();
        let moved = seen.clone();
        listen
            .listen::<bus::LayoutChanged>("test.moved", ListenOpts::new(), move |change, _| {
                if change.change == Change::TabMoved {
                    moved
                        .lock()
                        .unwrap()
                        .moved
                        .push((change.dock.clone(), change.zone.clone()));
                }
                Ok(())
            })
            .unwrap();
        (harness(space), seen)
    }

    fn named(dock: &str, zone: &str) -> (Option<String>, Option<String>) {
        (Some(dock.to_owned()), Some(zone.to_owned()))
    }

    #[test]
    #[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
    fn a_drop_on_a_single_dock_strip_names_the_zone() {
        let (mut harness, seen) = with_bus();
        let target = strip(&harness, "left");
        drop_at(&mut harness, "Preview", target);
        let seen = seen.lock().unwrap();
        assert!(
            seen.asked.contains(&named("left", "left")),
            "{:?}",
            seen.asked
        );
        assert_eq!(seen.moved, [named("left", "left")]);
    }

    #[test]
    #[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
    fn a_drop_on_a_split_strip_names_the_folded_zone() {
        let (mut harness, seen) = with_bus();
        let target = strip(&harness, "side");
        drop_at(&mut harness, "Preview", target);
        let seen = seen.lock().unwrap();
        assert!(
            seen.asked.contains(&named("top", "side")),
            "{:?}",
            seen.asked
        );
        assert_eq!(seen.moved, [named("top", "side")]);
    }

    #[test]
    #[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
    fn a_drop_in_an_open_dock_names_its_zone() {
        let (mut harness, seen) = with_bus();
        let target = harness.get_by_label("Editor").rect().center();
        drop_at(&mut harness, "Preview", target);
        let seen = seen.lock().unwrap();
        assert_eq!(seen.moved, [named("centre", "centre")]);
    }
}
