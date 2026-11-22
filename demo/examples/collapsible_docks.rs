#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! Collapsible docks (`Fold::Collapse`): a viewport between two side docks, over a bottom dock.
//! A side dock with no tab collapses to nothing and the viewport takes its room; drag a tab
//! from the bottom dock and a thin band shows at each collapsed dock's former edge. Release the
//! tab on a band and the dock comes back at its size, with the tab in it. Drag its last tab out
//! and it collapses again.

use eframe::NativeOptions;
use egui::{Ui, WidgetText};
use egui_stack::layout::{
    Content, Dir, Fold, Layout, LayoutArea, Size, Zone, ZoneName, ZoneViewer,
};
use egui_stack::{DockArea, DockRules, DockSpace, TabViewer};

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Collapsible docks",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

/// A side dock that collapses when empty.
fn side(name: &str) -> Zone {
    Zone::leaf(name, Content::Dock(name.into())).with(|c| {
        c.default = Some(Size::Points(240.0));
        c.min = Some(Size::Points(120.0));
        c.fold = Fold::Collapse;
    })
}

struct App {
    layout: Layout,
    space: DockSpace<String>,
}

impl App {
    fn new() -> Self {
        let layout = Layout::new(Zone::split(
            "root",
            Dir::Vertical,
            vec![
                Zone::split(
                    "main",
                    Dir::Horizontal,
                    vec![
                        side("left"),
                        Zone::leaf("viewport", Content::App).with(|c| {
                            c.grow = true;
                            c.min = Some(Size::Points(200.0));
                        }),
                        side("right"),
                    ],
                )
                .with(|c| c.grow = true),
                Zone::leaf("bottom", Content::Dock("bottom".into()))
                    .with(|c| c.default = Some(Size::Points(180.0))),
            ],
        ));
        let mut space = DockSpace::new();
        space.add_dock(
            "bottom",
            DockRules::default(),
            ["Outliner", "Properties", "Console", "Timeline"]
                .map(str::to_owned)
                .to_vec(),
        );
        space.add_dock("left", DockRules::default(), vec![]);
        space.add_dock("right", DockRules::default(), vec![]);
        Self { layout, space }
    }
}

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
        ui.label(format!("{tab}: drag me onto a collapsed dock's band."));
    }
}

struct Viewer<'a> {
    space: &'a mut DockSpace<String>,
}

impl ZoneViewer for Viewer<'_> {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        match zone.0.as_str() {
            "viewport" => {
                ui.centered_and_justified(|ui| ui.heading("Viewport"));
            }
            dock => DockArea::dock(self.space, dock).show_inside(ui, &mut Tabs),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        // Empty docks give their room to the viewport.
        self.space.collapse_empty(&mut self.layout);
        let laid = LayoutArea::new(&mut self.layout).show_inside(
            ui,
            &mut Viewer {
                space: &mut self.space,
            },
        );
        // Their bands take drops while a tab is dragged...
        self.space.bind(&laid);
        // ...and a dock a tab was dropped on comes back.
        for zone in self.space.take_unfolds() {
            self.layout.set_folded(&zone.0, false);
        }
    }
}
