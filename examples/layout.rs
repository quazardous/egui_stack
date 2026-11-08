#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! A window laid out by `egui_stack::layout`: a toolbar, two sides around a
//! centre that keeps its least size, a status bar; drag the sashes between
//! them, or focus one with Tab and use the arrow keys.

use eframe::NativeOptions;
use egui_stack::layout::{
    Content, Dir, Layout, LayoutArea, Priority, Size, Zone, ZoneName, ZoneViewer,
};

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Layout",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App { layout: layout() }))),
    )
}

fn layout() -> Layout {
    Layout::new(Zone::split(
        "root",
        Dir::Vertical,
        vec![
            Zone::leaf("toolbar", Content::App).with(|c| c.fit_content = true),
            Zone::split(
                "middle",
                Dir::Horizontal,
                vec![
                    Zone::leaf("left", Content::App).with(|c| {
                        c.min = Some(Size::Points(120.0));
                        c.max = Some(Size::Share(0.4));
                        c.default = Some(Size::Points(220.0));
                    }),
                    Zone::leaf("centre", Content::App).with(|c| {
                        c.grow = true;
                        c.min = Some(Size::Points(300.0));
                    }),
                    Zone::leaf("right", Content::App).with(|c| {
                        c.min = Some(Size::Points(100.0));
                        c.default = Some(Size::Points(180.0));
                        c.priority = Priority::Low;
                    }),
                ],
            )
            .with(|c| c.grow = true),
            Zone::leaf("status", Content::App).with(|c| c.default = Some(Size::Points(24.0))),
        ],
    ))
}

struct App {
    layout: Layout,
}

struct Viewer;

impl ZoneViewer for Viewer {
    fn ui(&mut self, ui: &mut egui::Ui, zone: &ZoneName) {
        ui.label(format!("{zone}"));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        LayoutArea::new(&mut self.layout).show_inside(ui, &mut Viewer);
    }
}
