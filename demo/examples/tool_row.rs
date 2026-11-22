#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! Toolbar rows: a formatting row across the top and a tool palette down the side.
//!
//! Narrow the window: the tools that do not fit go into the More button at the row's end. Tab
//! to a row, then move along it with the arrows, Home and End; Enter or Space clicks.

use eframe::{NativeOptions, egui};
use egui::{Color32, Painter, Rect, Stroke, vec2};
use egui_stack::toolbar::{Tool, ToolDisplay, ToolRow};

fn main() -> eframe::Result<()> {
    let options = NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Tool rows",
        options,
        Box::new(|_cc| Ok(Box::<ToolRowApp>::default())),
    )
}

#[derive(Default)]
struct ToolRowApp {
    display: ToolDisplay,
    bold: bool,
    italic: bool,
    align: usize,
    tool: usize,
    saving: bool,
    log: Vec<String>,
}

/// Icons drawn as shapes, so that the example needs no image files.
fn dot(painter: &Painter, rect: Rect, color: Color32) {
    painter.circle_filled(rect.center(), rect.width() / 3.0, color);
}

fn square(painter: &Painter, rect: Rect, color: Color32) {
    painter.rect_stroke(
        rect.shrink(2.0),
        1.0,
        Stroke::new(1.5, color),
        egui::StrokeKind::Inside,
    );
}

fn lines(painter: &Painter, rect: Rect, color: Color32) {
    for i in 0..3 {
        let y = rect.top() + rect.height() * (0.25 + 0.25 * i as f32);
        painter.hline(rect.x_range().shrink(2.0), y, Stroke::new(1.5, color));
    }
}

impl eframe::App for ToolRowApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("format").show(ui, |ui| {
            let aligns = ["Left", "Centre", "Right"];
            let mut tools = vec![
                Tool::button("new", "New").icon_painter(square),
                Tool::button("save", "Save")
                    .icon_painter(square)
                    .shortcut("Ctrl+S")
                    .busy(self.saving),
                Tool::separator(),
                Tool::button("cut", "Cut").icon_painter(lines),
                Tool::button("copy", "Copy").icon_painter(lines),
                Tool::button("paste", "Paste")
                    .icon_painter(lines)
                    .enabled(false)
                    .tooltip("Paste: the clipboard is empty"),
                Tool::separator(),
                Tool::toggle("bold", "Bold", self.bold).icon_painter(dot),
                Tool::toggle("italic", "Italic", self.italic).icon_painter(dot),
                Tool::separator(),
            ];
            for (i, name) in aligns.iter().enumerate() {
                tools.push(Tool::radio(*name, *name, self.align == i).icon_painter(lines));
            }
            tools.push(Tool::spacer());
            tools.push(Tool::menu("view", "View", |ui| {
                ui.label("A menu of the caller's own");
                let _ = ui.button("Zoom in");
                let _ = ui.button("Zoom out");
            }));
            let out = ToolRow::new("example.format")
                .title("Formatting")
                .display(self.display)
                .show(ui, tools);
            if let Some(id) = out.clicked {
                match id.as_str() {
                    "bold" => self.bold = !self.bold,
                    "italic" => self.italic = !self.italic,
                    "save" => self.saving = !self.saving,
                    other => {
                        if let Some(i) = aligns.iter().position(|a| *a == other) {
                            self.align = i;
                        }
                    }
                }
                self.log.push(id);
            }
        });
        egui::Panel::left("palette")
            .resizable(false)
            .show(ui, |ui| {
                let tools = ["Select", "Brush", "Eraser", "Text", "Crop"]
                    .iter()
                    .enumerate()
                    .map(|(i, name)| Tool::radio(*name, *name, self.tool == i).icon_painter(dot));
                let out = ToolRow::new("example.palette")
                    .title("Tools")
                    .vertical()
                    .show(ui, tools);
                if let Some(id) = out.clicked {
                    let names = ["Select", "Brush", "Eraser", "Text", "Crop"];
                    self.tool = names.iter().position(|n| *n == id).unwrap_or(0);
                    self.log.push(id);
                }
            });
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Show:");
                ui.radio_value(&mut self.display, ToolDisplay::Icons, "icons");
                ui.radio_value(&mut self.display, ToolDisplay::Beside, "icons and labels");
                ui.radio_value(&mut self.display, ToolDisplay::Text, "labels");
            });
            ui.add_space(8.0);
            ui.label("Clicked:");
            egui::ScrollArea::vertical()
                .max_height(ui.available_height() - 8.0)
                .show(ui, |ui| {
                    ui.set_min_size(vec2(ui.available_width(), 0.0));
                    for id in self.log.iter().rev() {
                        ui.monospace(id);
                    }
                });
        });
    }
}
