//! A `LayoutArea`'s names come from the viewer's titles and the labels; a
//! zone of fixed size has no sash; a dragged sash snaps to its zone's
//! default size and to another sash's line, with a guide, unless Alt is
//! held.

use std::cell::RefCell;

use egui::{Modifiers, Pos2, Ui, accesskit::Role, vec2};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::Labels;
use egui_stack::layout::{
    Content, Dir, Layout, LayoutArea, LayoutPainter, LayoutStyle, Size, SnapGuidePaint, SnapKind,
    Zone, ZoneName, ZoneViewer,
};

struct Viewer;

impl ZoneViewer for Viewer {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        ui.label(format!("Body of {zone}"));
    }

    fn title(&mut self, zone: &ZoneName) -> String {
        format!("The {zone} area")
    }
}

/// Records the guides painted.
#[derive(Default)]
struct Guides(RefCell<Vec<SnapKind>>);

impl LayoutPainter for Guides {
    fn snap_guide(&self, _painter: &egui::Painter, guide: &SnapGuidePaint<'_>) {
        self.0.borrow_mut().push(guide.kind);
    }
}

struct App {
    layout: Layout,
    snap: f32,
    guides: Vec<SnapKind>,
}

fn harness(layout: Layout) -> Harness<'static, App> {
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            let mut labels = Labels::default();
            labels.resize_zone = "Size of {zone}".into();
            let mut style = LayoutStyle::from_egui(ui.style());
            style.snap = app.snap;
            let painter = Guides::default();
            LayoutArea::new(&mut app.layout)
                .labels(labels)
                .style(style)
                .painter(&painter)
                .show_inside(ui, &mut Viewer);
            app.guides.extend(painter.0.take());
        },
        App {
            layout,
            snap: 8.0,
            guides: Vec::new(),
        },
    );
    harness.run();
    harness
}

/// A fixed 48-point bar, a 200-point side, a centre.
fn row() -> Layout {
    Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::leaf("bar", Content::App).with(|c| {
                c.min = Some(Size::Points(48.0));
                c.max = Some(Size::Points(48.0));
            }),
            Zone::leaf("left", Content::App).with(|c| {
                c.min = Some(Size::Points(100.0));
                c.default = Some(Size::Points(200.0));
            }),
            Zone::leaf("centre", Content::App).with(|c| c.grow = true),
        ],
    ))
}

/// Two rows, each a side and a centre: the top side 200 points, the bottom
/// one 260.
fn rows() -> Layout {
    let row = |name: &str, side: &str, default: f32, centre: &str| {
        Zone::split(
            name,
            Dir::Horizontal,
            vec![
                Zone::leaf(side, Content::App).with(|c| {
                    c.min = Some(Size::Points(100.0));
                    c.default = Some(Size::Points(default));
                }),
                Zone::leaf(centre, Content::App).with(|c| c.grow = true),
            ],
        )
    };
    Layout::new(Zone::split(
        "root",
        Dir::Vertical,
        vec![
            row("top", "a", 200.0, "b").with(|c| c.grow = true),
            row("bottom", "c", 260.0, "d").with(|c| c.default = Some(Size::Points(150.0))),
        ],
    ))
}

fn size(harness: &Harness<'_, App>, zone: &str) -> Option<f32> {
    harness.state().layout.zone(zone).unwrap().state.size
}

/// Drags the sash named `label` by `dx` points.
fn drag(harness: &mut Harness<'_, App>, label: &str, dx: f32) {
    let start: Pos2 = harness.get_by_label(label).rect().center();
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=4 {
        harness.hover_at(start + vec2(dx * step as f32 / 4.0, 0.0));
        harness.step();
    }
    harness.drop_at(start + vec2(dx, 0.0));
    harness.run();
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn zones_and_sashes_are_named_after_the_viewers_titles() {
    let harness = harness(row());
    let pane = harness.get_by_label("The left area");
    assert_eq!(pane.accesskit_node().role(), Role::Pane);
    // The zone's bare name is no longer a label: it cannot clash with the
    // application's own.
    assert!(harness.query_by_label("left").is_none());
    let sash = harness.get_by_label("Size of The left area");
    assert_eq!(sash.accesskit_node().role(), Role::Splitter);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_zone_of_fixed_size_has_no_sash() {
    let harness = harness(row());
    assert!(harness.query_by_label("Size of The bar area").is_none());
    assert_eq!(harness.get_all_by_role(Role::Splitter).count(), 1);
    assert_eq!(harness.get_by_label("The bar area").rect().width(), 48.0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dragged_sash_snaps_to_its_default_size_with_a_guide() {
    let mut harness = harness(row());
    drag(&mut harness, "Size of The left area", 40.0);
    assert_eq!(size(&harness, "left"), Some(240.0));
    harness.state_mut().guides.clear();

    // 204 is within 8 points of the default, 200.
    drag(&mut harness, "Size of The left area", -36.0);
    assert_eq!(size(&harness, "left"), Some(200.0));
    assert!(harness.state().guides.contains(&SnapKind::Default));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn snapping_off_or_alt_held_leaves_the_sash_where_dragged() {
    let mut harness = harness(row());
    // Drags in steps past egui's drag threshold, from 240 back near the
    // default, 200.
    harness.state_mut().snap = 0.0;
    drag(&mut harness, "Size of The left area", 40.0);
    drag(&mut harness, "Size of The left area", -36.0);
    assert_eq!(size(&harness, "left"), Some(204.0));

    harness.state_mut().snap = 8.0;
    drag(&mut harness, "Size of The left area", 36.0);
    assert_eq!(size(&harness, "left"), Some(240.0));
    harness.event(egui::Event::ModifiersChanged(Modifiers::ALT));
    harness.state_mut().guides.clear();
    drag(&mut harness, "Size of The left area", -34.0);
    harness.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    assert_eq!(size(&harness, "left"), Some(206.0));
    assert!(harness.state().guides.is_empty());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dragged_sash_lines_up_with_another_sash() {
    let mut harness = harness(rows());
    // The bottom side from 260 to 205: the top side's sash is at 200.
    drag(&mut harness, "Size of The c area", -55.0);
    assert_eq!(size(&harness, "c"), Some(200.0));
    assert!(harness.state().guides.contains(&SnapKind::Sash));
    let top = harness.get_by_label("The a area").rect().right();
    let bottom = harness.get_by_label("The c area").rect().right();
    assert_eq!(top, bottom);
}
