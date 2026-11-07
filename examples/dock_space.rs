//! A `DockSpace`: a dock in a left side panel, one in a right side panel and one in the centre,
//! each drawn by its own `DockArea::dock` call. A tab is dragged between them as inside one
//! dock: the edge bands of the leaf under the pointer split it, its middle adds the tab, a tab
//! of its bar takes the tab before it.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::{NativeOptions, egui};
use egui_stack::{
    AllowedSplits, DockArea, DockDrop, DockRules, DockSpace, tab_viewer::DropVerdict,
};

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "DockSpace",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

struct TabViewer;

impl egui_stack::TabViewer for TabViewer {
    type Tab = String;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::unique(tab.as_str())
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("Content of {tab}"));
    }

    // The editor stays in the centre: the other docks refuse it, with a reason.
    fn dock_drop_allowed(&self, tab: &Self::Tab, drop: &DockDrop<'_>) -> DropVerdict {
        if tab == "Editor" && drop.to != Some("centre") {
            DropVerdict::Refuse(Some("The editor stays in the centre".into()))
        } else {
            DropVerdict::Allow
        }
    }
}

struct MyApp {
    space: DockSpace<String>,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut space = DockSpace::new();
        // The left dock splits only into rows.
        space.add_dock(
            "left",
            DockRules::default().splits(AllowedSplits::TopBottomOnly),
            vec!["Files".to_owned(), "Search".to_owned()],
        );
        space.add_dock(
            "centre",
            DockRules::default(),
            vec!["Editor".to_owned(), "Preview".to_owned()],
        );
        // The right dock stays, empty, as a target once its tab leaves.
        space.add_dock("right", DockRules::default(), vec!["Outline".to_owned()]);
        Self { space }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The application lays the docks out; each is drawn where it is put.
        egui::Panel::left("left")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| {
                DockArea::dock(&mut self.space, "left").show_inside(ui, &mut TabViewer);
            });
        egui::Panel::right("right")
            .resizable(true)
            .default_size(220.0)
            .show(ui, |ui| {
                DockArea::dock(&mut self.space, "right").show_inside(ui, &mut TabViewer);
            });
        DockArea::dock(&mut self.space, "centre").show_inside(ui, &mut TabViewer);
    }
}
