//! A docked zone's content follows its sash in the same frame: while the
//! sash is dragged, the zone is drawn at the size the sash shows, not the
//! one it had the frame before.

use std::cell::Cell;

use egui::{Pos2, Ui, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::layout::{
    Content, Dir, Layout, LayoutArea, LayoutPainter, SashPaint, Size, Zone, ZoneName, ZoneViewer,
};

/// Records the right edge of the side zone's content rect.
struct Viewer<'a>(&'a Cell<f32>);

impl ZoneViewer for Viewer<'_> {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        if zone.0 == "side" {
            self.0.set(ui.max_rect().right());
        }
        ui.label(format!("Body of {zone}"));
    }

    fn title(&mut self, zone: &ZoneName) -> String {
        format!("The {zone} area")
    }
}

/// Records the middle of the side zone's sash as painted.
#[derive(Default)]
struct Sashes(Cell<f32>);

impl LayoutPainter for Sashes {
    fn sash(&self, _painter: &egui::Painter, sash: &SashPaint<'_>) {
        if sash.zone.0 == "side" {
            self.0.set(sash.rect.center().x);
        }
    }
}

#[derive(Default)]
struct Frame {
    content_right: f32,
    sash_x: f32,
}

struct App {
    layout: Layout,
    frame: Frame,
}

/// A 200-point side and a centre.
fn row() -> Layout {
    Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::leaf("side", Content::App).with(|c| {
                c.min = Some(Size::Points(100.0));
                c.default = Some(Size::Points(200.0));
            }),
            Zone::leaf("centre", Content::App).with(|c| c.grow = true),
        ],
    ))
}

fn harness() -> Harness<'static, App> {
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            let content = Cell::new(f32::NAN);
            let painter = Sashes(Cell::new(f32::NAN));
            LayoutArea::new(&mut app.layout)
                .painter(&painter)
                .show_inside(ui, &mut Viewer(&content));
            app.frame = Frame {
                content_right: content.get(),
                sash_x: painter.0.get(),
            };
        },
        App {
            layout: row(),
            frame: Frame::default(),
        },
    );
    harness.run();
    harness
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_content_and_the_sash_move_in_the_same_frame() {
    let mut harness = harness();
    let start: Pos2 = harness.get_by_label("Resize The side area").rect().center();
    let before = harness.state().frame.sash_x;
    assert_eq!(harness.state().frame.content_right, before);
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    let mut seen = Vec::new();
    for step in 1..=8 {
        harness.hover_at(start + vec2(15.0 * step as f32, 0.0));
        harness.step();
        let frame = &harness.state().frame;
        assert_eq!(
            frame.content_right, frame.sash_x,
            "step {step}: the content's edge trails the sash"
        );
        seen.push(frame.sash_x);
    }
    harness.drop_at(start + vec2(120.0, 0.0));
    harness.run();
    let frame = &harness.state().frame;
    assert_eq!(frame.sash_x, before + 120.0);
    assert_eq!(frame.content_right, frame.sash_x);
    // The drag did move the sash while it went on.
    assert!(seen.windows(2).filter(|w| w[0] != w[1]).count() >= 4);
}
