//! A `LayoutArea` draws its zones, and its sashes resize them within their
//! bounds: by drag, by the keyboard and through assistive technology.

use egui::{Key, Modifiers, Pos2, Ui, accesskit::Role, vec2};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::layout::{Content, Dir, Layout, LayoutArea, Size, Zone, ZoneName, ZoneViewer};

struct Viewer;

impl ZoneViewer for Viewer {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        ui.label(format!("Body of {zone}"));
    }
}

/// A 200-point side (100 to 300), a centre of at least 300, a 150-point
/// side on the right, in 800 points.
fn layout() -> Layout {
    Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::leaf("left", Content::App).with(|c| {
                c.min = Some(Size::Points(100.0));
                c.max = Some(Size::Points(300.0));
                c.default = Some(Size::Points(200.0));
            }),
            Zone::leaf("centre", Content::App).with(|c| {
                c.grow = true;
                c.min = Some(Size::Points(300.0));
            }),
            Zone::leaf("right", Content::App).with(|c| c.default = Some(Size::Points(150.0))),
        ],
    ))
}

fn harness() -> Harness<'static, Layout> {
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, layout: &mut Layout| {
            LayoutArea::new(layout).show_inside(ui, &mut Viewer);
        },
        layout(),
    );
    harness.run();
    harness
}

fn size(harness: &Harness<'_, Layout>, zone: &str) -> Option<f32> {
    harness.state().zone(zone).unwrap().state.size
}

fn sash_centre(harness: &Harness<'_, Layout>, zone: &str) -> Pos2 {
    harness
        .get_by_label(&format!("Resize {zone}"))
        .rect()
        .center()
}

/// Drags the sash of `zone` by `dx` points.
fn drag(harness: &mut Harness<'_, Layout>, zone: &str, dx: f32) {
    let start = sash_centre(harness, zone);
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
fn the_zones_are_drawn_as_panes() {
    let harness = harness();
    for zone in ["left", "centre", "right"] {
        let pane = harness.get_by_label(zone);
        assert_eq!(pane.accesskit_node().role(), Role::Pane);
        harness.get_by_label(&format!("Body of {zone}"));
    }
    assert_eq!(harness.get_by_label("left").rect().width(), 200.0);
    // The centre takes the rest; it has no sash of its own.
    assert_eq!(harness.get_all_by_role(Role::Splitter).count(), 2);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_sash_drag_resizes_its_zone_within_its_bounds() {
    let mut harness = harness();
    drag(&mut harness, "left", 40.0);
    assert_eq!(size(&harness, "left"), Some(240.0));
    assert_eq!(harness.get_by_label("left").rect().width(), 240.0);

    // Its largest size.
    drag(&mut harness, "left", 200.0);
    assert_eq!(size(&harness, "left"), Some(300.0));

    // Its least size.
    drag(&mut harness, "left", -400.0);
    assert_eq!(size(&harness, "left"), Some(100.0));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_sash_keeps_the_centres_least_size() {
    let mut harness = harness();
    // The harness's 784 points (its panel's margins off 800) less 200 and
    // 150 leave the centre 434: the right side grows by 134 at most.
    drag(&mut harness, "right", -400.0);
    assert_eq!(size(&harness, "right"), Some(284.0));
    assert_eq!(harness.get_by_label("centre").rect().width(), 300.0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_focused_sash_moves_with_the_arrows_and_enter_resets_it() {
    let mut harness = harness();
    harness.get_by_label("Resize left").focus();
    harness.run();
    assert!(harness.get_by_label("Resize left").is_focused());

    harness.key_press_modifiers(Modifiers::NONE, Key::ArrowRight);
    harness.run();
    assert_eq!(size(&harness, "left"), Some(216.0));

    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowLeft);
    harness.run();
    assert_eq!(size(&harness, "left"), Some(168.0));

    // The right sash grows its zone leftwards.
    harness.get_by_label("Resize right").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::NONE, Key::ArrowLeft);
    harness.run();
    assert_eq!(size(&harness, "right"), Some(166.0));

    harness.get_by_label("Resize left").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::NONE, Key::Enter);
    harness.run();
    assert_eq!(size(&harness, "left"), None);
    assert_eq!(harness.get_by_label("left").rect().width(), 200.0);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_sash_is_a_splitter_with_its_zones_size() {
    let mut harness = harness();
    let sash = harness.get_by_label("Resize left");
    assert_eq!(sash.accesskit_node().role(), Role::Splitter);
    assert_eq!(sash.accesskit_node().numeric_value(), Some(200.0));
    assert_eq!(sash.accesskit_node().min_numeric_value(), Some(100.0));
    assert_eq!(sash.accesskit_node().max_numeric_value(), Some(300.0));

    sash.increment_accesskit();
    harness.run();
    assert_eq!(size(&harness, "left"), Some(216.0));

    harness
        .get_by_label("Resize left")
        .set_value_accesskit(250.0);
    harness.run();
    assert_eq!(size(&harness, "left"), Some(250.0));
    assert_eq!(
        harness
            .get_by_label("Resize left")
            .accesskit_node()
            .numeric_value(),
        Some(250.0)
    );
}
