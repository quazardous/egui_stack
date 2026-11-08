//! A `DockSpace` bound to a layout: the zones that refuse drops, or hold no
//! dock, are no-drop zones for the dock's drag.

use egui::{Pos2, Ui, WidgetText, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::layout::{
    Content, Dir, Drops, Layout, LayoutArea, Size, Zone, ZoneName, ZoneViewer,
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
        match zone.0.as_str() {
            "status" => {
                ui.label("Status");
            }
            dock => {
                DockArea::dock(self.space, dock).show_inside(ui, &mut Tabs);
            }
        }
    }
}

struct App {
    layout: Layout,
    space: DockSpace<String>,
}

/// A left dock that refuses drops, a centre dock, a status bar.
fn harness() -> Harness<'static, App> {
    let mut space = DockSpace::new();
    space.add_dock("left", DockRules::default(), vec!["Files".to_owned()]);
    space.add_dock(
        "centre",
        DockRules::default(),
        vec!["Editor".to_owned(), "Preview".to_owned()],
    );
    let layout = Layout::new(Zone::split(
        "root",
        Dir::Vertical,
        vec![
            Zone::split(
                "middle",
                Dir::Horizontal,
                vec![
                    Zone::leaf("left", Content::Dock("left".into())).with(|c| {
                        c.default = Some(Size::Points(250.0));
                        c.drops = Drops::Refuse;
                    }),
                    Zone::leaf("centre", Content::Dock("centre".into())).with(|c| c.grow = true),
                ],
            )
            .with(|c| c.grow = true),
            Zone::leaf("status", Content::App).with(|c| c.default = Some(Size::Points(60.0))),
        ],
    ));
    let mut harness = Harness::builder().with_size([900.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            let laid = LayoutArea::new(&mut app.layout).show_inside(
                ui,
                &mut Viewer {
                    space: &mut app.space,
                },
            );
            app.space.bind(&laid);
        },
        App { layout, space },
    );
    harness.run();
    harness
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

/// Where `tab` is: its dock's name, `None` in a floating window.
fn dock_of(app: &App, tab: &str) -> Option<String> {
    let path = app.space.state().find_tab(&tab.to_owned()).unwrap();
    app.space.dock_name(path.surface).map(str::to_owned)
}

fn left_centre(harness: &Harness<'_, App>) -> Pos2 {
    let laid = harness.state().layout.solve(
        egui::Rect::from_min_size(Pos2::new(8.0, 8.0), vec2(884.0, 384.0)),
        &Default::default(),
    );
    laid.rect("left").unwrap().center()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_zone_accepting_drops_takes_the_tab() {
    let mut harness = harness();
    harness
        .state_mut()
        .layout
        .constrain("left", |c| c.drops = Drops::Accept);
    let target = left_centre(&harness);
    drop_at(&mut harness, "Preview", target);
    assert_eq!(dock_of(harness.state(), "Preview").as_deref(), Some("left"));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_released_over_a_refusing_zone_goes_back() {
    let mut harness = harness();
    let target = left_centre(&harness);
    drop_at(&mut harness, "Preview", target);
    assert_eq!(
        dock_of(harness.state(), "Preview").as_deref(),
        Some("centre")
    );
    assert_eq!(harness.state().space.state().iter_all_tabs().count(), 3);
    assert!(
        harness
            .state()
            .space
            .dock("left")
            .unwrap()
            .tabs()
            .all(|tab| tab == "Files")
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_released_over_a_zone_without_a_dock_goes_back() {
    let mut harness = harness();
    let status = harness.get_by_label("Status").rect();
    drop_at(&mut harness, "Preview", status.center() + vec2(300.0, 0.0));
    // Not a floating window: back in its dock.
    assert_eq!(
        dock_of(harness.state(), "Preview").as_deref(),
        Some("centre")
    );
}
