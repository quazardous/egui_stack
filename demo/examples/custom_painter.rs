#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! Tabs painted as underlined labels and thin separators, by a `DockPainter`; the drop overlay
//! keeps the style's painting.

use eframe::{NativeOptions, egui};
use egui::{Painter, Stroke, epaint::TextShape};
use egui_stack::{
    DockArea, DockPainter, DockState, NodeIndex, PartState, SeparatorPaint, TabBarPaint,
    TabMeasure, TabPaint,
};

fn main() -> eframe::Result<()> {
    let options = NativeOptions::default();
    eframe::run_native(
        "Custom painter",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

/// Underlined tabs: the selected one in the accent colour, the others when hovered.
struct Underlined {
    accent: egui::Color32,
    text: egui::Color32,
    weak: egui::Color32,
    line: egui::Color32,
}

impl DockPainter for Underlined {
    fn tab_width(&self, tab: &TabMeasure<'_>) -> f32 {
        // 6 points either side of the title, narrower than the style's 8.
        tab.title.size().x + 12.0 + tab.close_width
    }

    fn tab_bar(&self, painter: &Painter, bar: &TabBarPaint<'_>) {
        let px = painter.pixels_per_point().recip();
        painter.hline(bar.rect.x_range(), bar.rect.bottom() - px, (px, self.line));
    }

    fn tab_bar_tail(&self, _painter: &Painter, _tail: &egui_stack::TabBarTailPaint<'_>) {}

    fn tab(&self, painter: &Painter, tab: &TabPaint<'_>) {
        let selected = tab.state.contains(PartState::SELECTED);
        let hovered = tab.state.contains(PartState::HOVERED);
        let color = if selected { self.text } else { self.weak };
        let pos = tab.rect.left_center() + egui::vec2(6.0, -tab.title.size().y / 2.0);
        painter.add(TextShape::new(pos, tab.title.clone(), color));
        if selected || hovered {
            let line = if selected { self.accent } else { self.weak };
            painter.hline(tab.rect.x_range(), tab.rect.bottom() - 1.0, (2.0, line));
        }
        if let Some(close) = tab.close {
            let lit = close.state.contains(PartState::HOVERED);
            let x = close.rect.shrink(close.rect.width() / 3.0);
            let stroke = Stroke::new(1.0, if lit { self.text } else { self.weak });
            painter.line_segment([x.left_top(), x.right_bottom()], stroke);
            painter.line_segment([x.right_top(), x.left_bottom()], stroke);
        }
    }

    fn separator(&self, painter: &Painter, separator: &SeparatorPaint<'_>) {
        let active = separator.state.contains(PartState::HOVERED)
            || separator.state.contains(PartState::DRAGGED);
        let color = if active { self.accent } else { self.line };
        painter.rect_filled(separator.rect, 0.0, color);
    }
}

struct TabViewer {}

impl egui_stack::TabViewer for TabViewer {
    type Tab = String;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::unique(tab)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("Content of {tab}"));
    }
}

struct MyApp {
    tree: DockState<String>,
}

impl Default for MyApp {
    fn default() -> Self {
        let mut tree = DockState::new(vec!["Scene".to_owned(), "Game".to_owned()]);
        let [a, _] = tree.main_surface_mut().split_left(
            NodeIndex::root(),
            0.3,
            vec!["Outline".to_owned(), "Assets".to_owned()],
        );
        tree.main_surface_mut()
            .split_below(a, 0.6, vec!["Inspector".to_owned()]);
        Self { tree }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let visuals = ui.visuals();
        let painter = Underlined {
            accent: visuals.selection.bg_fill,
            text: visuals.strong_text_color(),
            weak: visuals.weak_text_color(),
            line: visuals.widgets.noninteractive.bg_stroke.color,
        };
        DockArea::new(&mut self.tree)
            .painter(&painter)
            .show_inside(ui, &mut TabViewer {});
    }
}
