//! The built-in title of folded strips (`LayoutStyle::strip_title`): which
//! way it reads, how it is cut, how broad the strip grows, and the hook
//! that draws it instead (`ZoneViewer::strip_title`).

use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

use egui::epaint::TextShape;
use egui::{Pos2, Rect, Shape, Ui, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::layout::{
    Content, Dir, Fold, Layout, LayoutArea, LayoutStyle, Size, StripOrientation, StripTitle,
    VerticalReading, Zone, ZoneName, ZoneViewer,
};

#[derive(Default)]
struct Viewer {
    /// Titles other than the zone's name.
    titles: HashMap<String, String>,
    /// The icon of every strip.
    icon: Option<String>,
    /// Whether the viewer draws the titles itself.
    custom: bool,
    /// What the hook was given.
    hooked: Vec<(String, Rect, StripOrientation)>,
}

impl ZoneViewer for Viewer {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        ui.label(format!("Body of {zone}"));
    }

    fn title(&mut self, zone: &ZoneName) -> String {
        self.titles
            .get(&zone.0)
            .cloned()
            .unwrap_or_else(|| zone.0.clone())
    }

    fn strip_title(
        &mut self,
        ui: &mut Ui,
        zone: &ZoneName,
        rect: Rect,
        orientation: StripOrientation,
    ) -> bool {
        if self.custom {
            ui.painter().rect_filled(rect, 0.0, egui::Color32::RED);
            self.hooked.push((zone.0.clone(), rect, orientation));
        }
        self.custom
    }

    fn strip_icon(&mut self, _zone: &ZoneName) -> Option<String> {
        self.icon.clone()
    }
}

struct App {
    layout: Layout,
    style: LayoutStyle,
    viewer: Viewer,
    strips: HashMap<String, Rect>,
}

fn side(name: &str) -> Zone {
    Zone::leaf(name, Content::App).with(|c| {
        c.min = Some(Size::Points(100.0));
        c.default = Some(Size::Points(200.0));
        c.fold = Fold::ToStrip;
    })
}

/// A row of a left side, a centre and a right side, over a bottom zone:
/// all three folded.
fn layout() -> Layout {
    let mut layout = Layout::new(Zone::split(
        "root",
        Dir::Vertical,
        vec![
            Zone::split(
                "row",
                Dir::Horizontal,
                vec![
                    side("left"),
                    Zone::leaf("centre", Content::App).with(|c| c.grow = true),
                    side("right"),
                ],
            )
            .with(|c| c.grow = true),
            side("bottom"),
        ],
    ));
    for zone in ["left", "right", "bottom"] {
        layout.set_folded(zone, true);
    }
    layout
}

fn titled() -> LayoutStyle {
    let mut style = LayoutStyle::default();
    style.strip_title = Some(StripTitle::default());
    style
}

fn harness(style: LayoutStyle, viewer: Viewer, height: f32) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size([800.0, height])
        .build_ui_state(
            |ui, app: &mut App| {
                ui.ctx().global_style_mut(|style| {
                    style.interaction.tooltip_delay = 0.0;
                    style.interaction.show_tooltips_only_when_still = false;
                });
                app.viewer.hooked.clear();
                let laid = LayoutArea::new(&mut app.layout)
                    .style(app.style.clone())
                    .show_inside(ui, &mut app.viewer);
                app.strips = laid
                    .zones
                    .iter()
                    .filter_map(|zone| zone.strip.map(|strip| (zone.name.0.clone(), strip)))
                    .collect();
            },
            App {
                layout: layout(),
                style,
                viewer,
                strips: HashMap::new(),
            },
        );
    harness.run();
    harness
}

fn texts(shape: &Shape, out: &mut Vec<TextShape>) {
    match shape {
        Shape::Text(text) => out.push(text.clone()),
        Shape::Vec(shapes) => shapes.iter().for_each(|shape| texts(shape, out)),
        _ => {}
    }
}

/// The text shapes painted this frame whose text starts with `start`.
fn painted(harness: &Harness<'_, App>, start: &str) -> Vec<TextShape> {
    let mut all = Vec::new();
    for clipped in &harness.output().shapes {
        texts(&clipped.shape, &mut all);
    }
    all.retain(|text| text.galley.text().starts_with(start));
    all
}

fn strip(harness: &Harness<'_, App>, zone: &str) -> Rect {
    harness.state().strips[zone]
}

/// The one title painted for `zone`, inside its strip and past its grip.
fn title(harness: &Harness<'_, App>, zone: &str) -> TextShape {
    let shapes = painted(harness, zone);
    assert_eq!(shapes.len(), 1, "{zone}: one title");
    let shape = shapes.into_iter().next().unwrap();
    let strip = strip(harness, zone);
    let bounds = shape.visual_bounding_rect();
    assert!(
        strip.expand(0.5).contains_rect(bounds),
        "{zone}: {bounds:?} in {strip:?}"
    );
    let breadth = strip.width().min(strip.height());
    if strip.height() > strip.width() {
        assert!(
            bounds.top() >= strip.top() + breadth,
            "{zone}: past the grip"
        );
    } else {
        assert!(
            bounds.left() >= strip.left() + breadth,
            "{zone}: past the grip"
        );
    }
    shape
}

fn click(harness: &mut Harness<'_, App>, pos: Pos2) {
    harness.hover_at(pos);
    harness.step();
    harness.drag_at(pos);
    harness.step();
    harness.drop_at(pos);
    harness.run();
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn strips_show_no_title_unless_asked() {
    let harness = harness(LayoutStyle::default(), Viewer::default(), 400.0);
    for zone in ["left", "right", "bottom"] {
        assert!(painted(&harness, zone).is_empty(), "{zone}");
    }
    assert_eq!(strip(&harness, "left").width(), 24.0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn side_strips_read_the_same_way_bottom_up_by_default() {
    let harness = harness(titled(), Viewer::default(), 400.0);
    assert_eq!(title(&harness, "left").angle, -FRAC_PI_2);
    assert_eq!(title(&harness, "right").angle, -FRAC_PI_2);
    assert_eq!(title(&harness, "bottom").angle, 0.0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn side_strips_can_read_top_down() {
    let mut style = titled();
    style.strip_title.as_mut().unwrap().vertical = VerticalReading::TopToBottom;
    let harness = harness(style, Viewer::default(), 400.0);
    assert_eq!(title(&harness, "left").angle, FRAC_PI_2);
    assert_eq!(title(&harness, "right").angle, FRAC_PI_2);
    assert_eq!(title(&harness, "bottom").angle, 0.0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_long_title_is_cut_and_shown_whole_in_a_tooltip() {
    let long = "left: a title far too long for the height of its strip";
    let viewer = Viewer {
        titles: HashMap::from([("left".to_owned(), long.to_owned())]),
        ..Viewer::default()
    };
    let mut harness = harness(titled(), viewer, 200.0);
    // Cut to its strip (`title` checks it fits), with an ellipsis.
    let shape = title(&harness, "left");
    assert!(shape.galley.elided);
    let last = shape.galley.rows.last().unwrap();
    assert_eq!(last.row.glyphs.last().unwrap().chr, '\u{2026}');
    assert!(harness.query_by_label(long).is_none());

    // Hovering the strip shows it whole.
    let strip = strip(&harness, "left");
    harness.hover_at(strip.center() + vec2(0.0, 20.0));
    harness.run();
    assert!(harness.query_by_label(long).is_some());

    // A title that fits is whole.
    assert!(!title(&harness, "right").galley.elided);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_strip_is_broad_enough_for_the_font_the_padding_and_the_icon() {
    let mut style = titled();
    let title = style.strip_title.as_mut().unwrap();
    title.font = egui::FontId::proportional(30.0);
    title.padding = 6.0;
    let harness = harness(style, Viewer::default(), 400.0);
    let row = harness
        .ctx
        .fonts_mut(|fonts| fonts.row_height(&egui::FontId::proportional(30.0)));
    // Side and bottom strips alike.
    for breadth in [
        strip(&harness, "left").width(),
        strip(&harness, "right").width(),
        strip(&harness, "bottom").height(),
    ] {
        assert!((breadth - (row + 12.0)).abs() < 1.0, "{breadth} for {row}");
    }
    // The centre keeps the rest.
    assert!(harness.get_by_label("centre").rect().left() >= strip(&harness, "left").right());

    // An icon wider than a line widens the strips, and stands upright
    // next to the grip.
    let viewer = Viewer {
        icon: Some("WWWWW".to_owned()),
        ..Viewer::default()
    };
    let harness = self::harness(titled(), viewer, 400.0);
    let icon = painted(&harness, "WWWWW");
    assert_eq!(icon.len(), 3);
    assert!(icon.iter().all(|icon| icon.angle == 0.0));
    let width = icon[0].galley.size().x;
    assert!(strip(&harness, "left").width() >= width + 8.0);
    assert!(strip(&harness, "left").width() > 24.0);
    title_after_icon(&harness);
}

fn title_after_icon(harness: &Harness<'_, App>) {
    let strip = strip(harness, "left");
    let icon = painted(harness, "WWWWW")
        .into_iter()
        .find(|icon| strip.contains_rect(icon.visual_bounding_rect()))
        .unwrap();
    let text = title(harness, "left").visual_bounding_rect();
    assert!(text.top() >= icon.visual_bounding_rect().bottom());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_hook_draws_the_title_in_the_room_past_the_grip() {
    let viewer = Viewer {
        custom: true,
        ..Viewer::default()
    };
    let mut harness = harness(titled(), viewer, 400.0);
    let mut hooked = harness.state().viewer.hooked.clone();
    hooked.sort_by(|a, b| a.0.cmp(&b.0));
    let orientations: Vec<_> = hooked
        .iter()
        .map(|(zone, _, o)| (zone.as_str(), *o))
        .collect();
    assert_eq!(
        orientations,
        [
            ("bottom", StripOrientation::Horizontal),
            ("left", StripOrientation::BottomToTop),
            ("right", StripOrientation::BottomToTop),
        ]
    );
    for (zone, rect, _) in &hooked {
        let strip = strip(&harness, zone);
        let breadth = strip.width().min(strip.height());
        assert!(strip.contains_rect(*rect), "{zone}");
        assert!(rect.left() >= strip.left() + 4.0 && rect.top() >= strip.top() + 4.0);
        if strip.height() > strip.width() {
            assert!(rect.top() >= strip.top() + breadth, "{zone}: past the grip");
        } else {
            assert!(
                rect.left() >= strip.left() + breadth,
                "{zone}: past the grip"
            );
        }
    }
    // The layout paints no title of its own...
    for zone in ["left", "right", "bottom"] {
        assert!(painted(&harness, zone).is_empty(), "{zone}");
    }
    // ...and the strip still unfolds on a click over the title.
    let rect = hooked[1].1;
    click(&mut harness, rect.center());
    assert!(!harness.state().layout.zone("left").unwrap().state.folded);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_titled_strip_clicks_drags_and_takes_the_keyboard_as_before() {
    let mut harness = harness(titled(), Viewer::default(), 400.0);
    // A click on the title unfolds.
    let at = title(&harness, "left").visual_bounding_rect().center();
    click(&mut harness, at);
    assert!(!harness.state().layout.zone("left").unwrap().state.folded);

    // The grip is still a button, named after the zone.
    harness.get_by_label("Unfold right").click();
    harness.run();
    assert!(!harness.state().layout.zone("right").unwrap().state.folded);

    // The edge still drags the zone out.
    let strip = strip(&harness, "bottom");
    let edge = strip.center_top();
    harness.hover_at(edge);
    harness.step();
    harness.drag_at(edge);
    harness.step();
    for step in 1..=4 {
        harness.hover_at(edge - vec2(0.0, 40.0 * step as f32));
        harness.step();
    }
    harness.drop_at(edge - vec2(0.0, 160.0));
    harness.run();
    assert!(!harness.state().layout.zone("bottom").unwrap().state.folded);
}
