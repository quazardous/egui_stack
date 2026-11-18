#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! Folded strips with titles (`LayoutStyle::strip_title`): fold the sides and the bottom with
//! their grips, or drag their sashes past the fold threshold. Left and right strips read the same
//! way; a long title is cut and shown whole on hover. The right side draws its own title through
//! `ZoneViewer::strip_title`, and the layout keeps the clicks, the grip and the drag.

use eframe::NativeOptions;
use egui::{Color32, CornerRadius, Rect, Ui};
use egui_stack::layout::{
    Content, Dir, Fold, Layout, LayoutArea, LayoutStyle, Size, StripOrientation, StripTitle,
    VerticalReading, Zone, ZoneName, ZoneViewer,
};

fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Strip titles",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

fn side(name: &str) -> Zone {
    Zone::leaf(name, Content::App).with(|c| {
        c.min = Some(Size::Points(120.0));
        c.default = Some(Size::Points(220.0));
        c.fold = Fold::ToStrip;
    })
}

struct App {
    layout: Layout,
    reading: VerticalReading,
}

impl App {
    fn new() -> Self {
        let mut layout = Layout::new(Zone::split(
            "root",
            Dir::Vertical,
            vec![
                Zone::split(
                    "row",
                    Dir::Horizontal,
                    vec![
                        side("project"),
                        Zone::leaf("centre", Content::App).with(|c| c.grow = true),
                        side("outline"),
                    ],
                )
                .with(|c| c.grow = true),
                side("terminal"),
            ],
        ));
        for zone in ["project", "outline", "terminal"] {
            layout.set_folded(zone, true);
        }
        Self {
            layout,
            reading: VerticalReading::BottomToTop,
        }
    }
}

struct Viewer<'a> {
    reading: &'a mut VerticalReading,
}

impl ZoneViewer for Viewer<'_> {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        if zone.0 == "centre" {
            ui.heading("Strip titles");
            ui.label("Side strips read:");
            ui.radio_value(self.reading, VerticalReading::BottomToTop, "bottom to top");
            ui.radio_value(self.reading, VerticalReading::TopToBottom, "top to bottom");
        } else {
            ui.label(self.title(zone));
        }
    }

    fn title(&mut self, zone: &ZoneName) -> String {
        match zone.0.as_str() {
            "project" => "Project files and folders of the workspace".to_owned(),
            "outline" => "Outline".to_owned(),
            "terminal" => "Terminal".to_owned(),
            other => other.to_owned(),
        }
    }

    fn strip_icon(&mut self, zone: &ZoneName) -> Option<String> {
        (zone.0 == "project").then(|| "\u{1F4C1}".to_owned())
    }

    /// The outline draws its own title: a coloured badge, turned like the others.
    fn strip_title(
        &mut self,
        ui: &mut Ui,
        zone: &ZoneName,
        rect: Rect,
        orientation: StripOrientation,
    ) -> bool {
        if zone.0 != "outline" {
            return false;
        }
        let badge = match orientation {
            StripOrientation::Horizontal => rect.with_max_x(rect.left() + 60.0),
            _ => rect.with_max_y(rect.top() + 60.0),
        };
        let badge = badge.intersect(rect);
        ui.painter().rect_filled(
            badge,
            CornerRadius::same(3),
            Color32::from_rgb(70, 110, 180),
        );
        ui.painter().text(
            badge.center(),
            egui::Align2::CENTER_CENTER,
            "O",
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
        true
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let mut style = LayoutStyle::from_egui(ui.style());
        let mut title = StripTitle::from_egui(ui.style());
        title.vertical = self.reading;
        style.strip_title = Some(title);
        LayoutArea::new(&mut self.layout).style(style).show_inside(
            ui,
            &mut Viewer {
                reading: &mut self.reading,
            },
        );
    }
}
