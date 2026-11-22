#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! Collapsible sections stacked in a side panel.
//!
//! Click a band to fold or open its section, drag the line above a band to resize, hold Ctrl
//! (Cmd on macOS) while dragging to pull the sections below as a block, and double-click a line
//! to share the room again.

use eframe::{NativeOptions, egui};
use egui_stack::sections::{Section, Sections, row_height};

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Sections",
        options,
        Box::new(|_cc| Ok(Box::<SectionsApp>::default())),
    )
}

struct SectionsApp {
    open_files: Vec<String>,
    grab_titles: bool,
    keep_titles: bool,
}

impl Default for SectionsApp {
    fn default() -> Self {
        Self {
            open_files: ["main.rs", "lib.rs", "sections.rs"]
                .map(str::to_owned)
                .to_vec(),
            grab_titles: false,
            keep_titles: false,
        }
    }
}

impl eframe::App for SectionsApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::left("side")
            .default_size(260.0)
            .show(ui, |ui| {
                let row = row_height(ui);
                let files = &self.open_files;
                let out = Sections::new("example.side")
                    .grab_titles(self.grab_titles)
                    .keep_titles(self.keep_titles)
                    .show(ui, |stack| {
                        stack.section(
                            Section::new("open", "Open editors").count(files.len() as u64),
                            move |ui| {
                                for file in files {
                                    ui.label(file);
                                }
                            },
                        );
                        stack.rows(
                            Section::new("log", "Log").count(10_000),
                            10_000,
                            move |ui, range| {
                                for i in range {
                                    let width = ui.available_width();
                                    ui.add_sized(
                                        [width, row],
                                        egui::Label::new(format!("Line {i}")),
                                    );
                                }
                            },
                        );
                        stack.section(Section::new("outline", "Outline").open(false), |ui| {
                            ui.weak("No symbols");
                        });
                    });
                if let Some((key, open)) = out.toggled {
                    println!("{key} {}", if open { "opened" } else { "folded" });
                }
            });
        egui::CentralPanel::default().show(ui, |ui| {
            ui.checkbox(&mut self.grab_titles, "Titles drag too");
            ui.checkbox(&mut self.keep_titles, "Titles stay when folding");
        });
    }
}
