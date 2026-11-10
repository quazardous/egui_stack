//! A zone folds to a strip and unfolds from it, lies over its neighbour
//! until Escape or a click outside, and shows a flyout; its state saves
//! with the layout.

use egui::{Key, Pos2, Ui, accesskit::Role, pos2, vec2};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::layout::{
    Content, Dir, Dismiss, Flyout, Fold, Layout, LayoutArea, LayoutEvent, Size, Zone, ZoneName,
    ZoneViewer,
};

struct Viewer;

impl ZoneViewer for Viewer {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        ui.label(format!("Body of {zone}"));
    }

    fn flyout(&mut self, ui: &mut Ui, flyout: &Flyout) {
        ui.label(format!("Flyout {}", flyout.key));
    }
}

struct App {
    layout: Layout,
    flyout: Option<Flyout>,
    events: Vec<LayoutEvent>,
}

/// A 200-point side that folds (least 100), and a centre.
fn layout() -> Layout {
    Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::leaf("left", Content::App).with(|c| {
                c.min = Some(Size::Points(100.0));
                c.default = Some(Size::Points(200.0));
                c.fold = Fold::ToStrip;
            }),
            Zone::leaf("centre", Content::App).with(|c| c.grow = true),
        ],
    ))
}

fn harness(layout: Layout) -> Harness<'static, App> {
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            let laid = LayoutArea::new(&mut app.layout)
                .flyout(app.flyout.clone())
                .show_inside(ui, &mut Viewer);
            for event in laid.events {
                if matches!(event, LayoutEvent::FlyoutClosed { .. }) {
                    app.flyout = None;
                }
                app.events.push(event);
            }
        },
        App {
            layout,
            flyout: None,
            events: Vec::new(),
        },
    );
    harness.run();
    harness
}

fn state(harness: &Harness<'_, App>) -> (bool, bool) {
    let state = harness.state().layout.zone("left").unwrap().state;
    (state.folded, state.over)
}

fn click(harness: &mut Harness<'_, App>, pos: Pos2) {
    harness.hover_at(pos);
    harness.step();
    harness.drag_at(pos);
    harness.step();
    harness.drop_at(pos);
    harness.run();
}

fn centre_point(harness: &Harness<'_, App>) -> Pos2 {
    harness.get_by_label("centre").rect().right_center() - vec2(50.0, 0.0)
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_sash_dragged_under_half_the_least_size_folds_and_the_grip_unfolds() {
    let mut harness = harness(layout());
    let start = harness.get_by_label("Resize left").rect().center();
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=4 {
        harness.hover_at(start - vec2(40.0 * step as f32, 0.0));
        harness.step();
    }
    harness.drop_at(start - vec2(160.0, 0.0));
    harness.run();
    assert_eq!(state(&harness), (true, false));
    // A strip in the flow; the centre takes the rest.
    assert_eq!(
        harness.get_by_label("Unfold left").accesskit_node().role(),
        Role::Button
    );
    let centre = harness.get_by_label("centre").rect();
    assert_eq!(centre.left(), 8.0 + 24.0);
    assert!(harness.query_by_label("Body of left").is_none());

    harness.get_by_label("Unfold left").click();
    harness.run();
    assert_eq!(state(&harness), (false, false));
    assert_eq!(harness.get_by_label("left").rect().width(), 200.0);
    assert!(
        harness
            .state()
            .events
            .iter()
            .any(|e| matches!(e, LayoutEvent::Unfolded { .. }))
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_zone_lying_over_closes_on_escape_or_a_click_outside() {
    let mut folded = layout();
    folded.set_over("left", true);
    let mut harness = harness(folded);
    // Its body is over the centre, next to its strip; the centre keeps its
    // room.
    let body = harness.get_by_label("Body of left").rect();
    assert!(body.left() >= 8.0 + 24.0);
    assert_eq!(harness.get_by_label("centre").rect().left(), 8.0 + 24.0);

    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(state(&harness), (true, false));
    assert!(harness.query_by_label("Body of left").is_none());

    harness.state_mut().layout.set_over("left", true);
    harness.run();
    // A click inside keeps it open.
    click(&mut harness, pos2(100.0, 200.0));
    assert_eq!(state(&harness), (false, true));
    let outside = centre_point(&harness);
    click(&mut harness, outside);
    assert_eq!(state(&harness), (true, false));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_flyout_opens_by_the_strip_and_closes_on_escape_or_a_click_outside() {
    let mut folded = layout();
    folded.set_folded("left", true);
    let mut harness = harness(folded);
    harness.state_mut().flyout = Some(Flyout::new("files", "left"));
    harness.run();
    let flyout = harness.get_by_label("Flyout files").rect();
    assert!(flyout.left() >= 8.0 + 24.0);

    // A click inside keeps it.
    click(&mut harness, flyout.center());
    assert!(harness.state().flyout.is_some());

    // Its free edge resizes it, and the breadth is kept.
    let edge = harness.get_by_label("Resize files").rect().center();
    harness.hover_at(edge);
    harness.step();
    harness.drag_at(edge);
    harness.step();
    for step in 1..=4 {
        harness.hover_at(edge + vec2(15.0 * step as f32, 0.0));
        harness.step();
    }
    harness.drop_at(edge + vec2(60.0, 0.0));
    harness.run();
    let kept = harness.state().layout.flyouts.get("files").copied();
    assert_eq!(kept, Some(300.0));

    let outside = centre_point(&harness);
    click(&mut harness, outside);
    assert!(harness.state().flyout.is_none());
    assert!(harness.query_by_label("Flyout files").is_none());

    harness.state_mut().flyout = Some(Flyout::new("files", "left"));
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().flyout.is_none());
}

#[cfg(feature = "serde")]
#[test]
fn zone_states_and_flyout_breadths_save_and_load() {
    let mut saved = layout();
    saved.set_over("left", true);
    saved.zone_mut("left").unwrap().state.size = Some(180.0);
    saved.flyouts.insert("files".to_owned(), 260.0);
    let json = serde_json::to_string(&saved).unwrap();
    let mut loaded: Layout = serde_json::from_str(&json).unwrap();
    loaded.reconcile(&layout());
    assert_eq!(loaded, saved);

    saved.set_folded("left", true);
    let json = serde_json::to_string(&saved).unwrap();
    let mut loaded: Layout = serde_json::from_str(&json).unwrap();
    loaded.reconcile(&layout());
    let state = loaded.zone("left").unwrap().state;
    assert!(state.folded && !state.over);
}

/// Drags from `from` by `dx` points.
fn drag_by(harness: &mut Harness<'_, App>, from: Pos2, dx: f32) {
    harness.hover_at(from);
    harness.step();
    harness.drag_at(from);
    harness.step();
    for step in 1..=4 {
        harness.hover_at(from + vec2(dx * step as f32 / 4.0, 0.0));
        harness.step();
    }
    harness.drop_at(from + vec2(dx, 0.0));
    harness.run();
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_strip_dragged_out_unfolds_its_zone_at_the_size_dragged_to() {
    let mut folded = layout();
    folded.set_folded("left", true);
    let mut harness = harness(folded);
    // The strip's edge facing the centre, below the grip.
    let edge = pos2(8.0 + 24.0, 200.0);

    // Under half the least size (50): it stays folded.
    drag_by(&mut harness, edge, 10.0);
    assert_eq!(state(&harness), (true, false));

    drag_by(&mut harness, edge, 150.0);
    assert_eq!(state(&harness), (false, false));
    let size = harness.state().layout.zone("left").unwrap().state.size;
    assert_eq!(size, Some(174.0));
    assert_eq!(harness.get_by_label("left").rect().width(), 174.0);
    assert!(
        harness
            .state()
            .events
            .iter()
            .any(|e| matches!(e, LayoutEvent::Unfolded { .. }))
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_grips_take_the_translated_labels() {
    let mut folded = layout();
    folded.set_folded("left", true);
    let mut labels = egui_stack::Labels::default();
    labels.unfold_zone = "Déplier {zone}".into();
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        move |ui, layout: &mut Layout| {
            LayoutArea::new(layout)
                .labels(labels.clone())
                .show_inside(ui, &mut Viewer);
        },
        folded,
    );
    harness.run();
    assert_eq!(
        harness.get_by_label("Déplier left").accesskit_node().role(),
        Role::Button
    );
}

/// The app's dismiss settings, and an overlay of its own (an egui area at
/// the window's top right) whose presses count as inside.
struct Hosted {
    app: App,
    flyout_dismiss: Dismiss,
    over_dismiss: Dismiss,
}

const OVERLAY: &str = "host overlay";

fn hosted(layout: Layout, flyout: Dismiss, over: Dismiss) -> Harness<'static, Hosted> {
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, host: &mut Hosted| {
            egui::Area::new(egui::Id::unique(OVERLAY))
                .order(egui::Order::Foreground)
                .fixed_pos(pos2(600.0, 20.0))
                .show(ui.ctx(), |ui| {
                    ui.allocate_exact_size(vec2(150.0, 60.0), egui::Sense::click());
                    ui.label(OVERLAY);
                });
            let inside = |_pos: Pos2, layer: Option<egui::LayerId>| {
                layer.is_some_and(|l| l.id == egui::Id::unique(OVERLAY))
            };
            let app = &mut host.app;
            let laid = LayoutArea::new(&mut app.layout)
                .flyout(app.flyout.clone())
                .flyout_dismiss(host.flyout_dismiss)
                .over_dismiss(host.over_dismiss)
                .inside(&inside)
                .show_inside(ui, &mut Viewer);
            for event in laid.events {
                if matches!(event, LayoutEvent::FlyoutClosed { .. }) {
                    app.flyout = None;
                }
                app.events.push(event);
            }
        },
        Hosted {
            app: App {
                layout,
                flyout: None,
                events: Vec::new(),
            },
            flyout_dismiss: flyout,
            over_dismiss: over,
        },
    );
    harness.run();
    harness
}

fn hosted_click(harness: &mut Harness<'_, Hosted>, pos: Pos2) {
    harness.hover_at(pos);
    harness.step();
    harness.drag_at(pos);
    harness.step();
    harness.drop_at(pos);
    harness.run();
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_zone_lying_over_set_never_dismissed_stays_until_its_grip_folds_it() {
    let mut over = layout();
    over.set_over("left", true);
    let mut harness = hosted(over, Dismiss::ANY, Dismiss::NEVER);
    harness.key_press(Key::Escape);
    harness.run();
    let state = |h: &Harness<'_, Hosted>| {
        let s = h.state().app.layout.zone("left").unwrap().state;
        (s.folded, s.over)
    };
    assert_eq!(state(&harness), (false, true));
    hosted_click(&mut harness, pos2(500.0, 300.0));
    assert_eq!(state(&harness), (false, true));
    assert!(harness.query_by_label("Body of left").is_some());

    harness.get_by_label("Fold left").click();
    harness.run();
    assert_eq!(state(&harness), (true, false));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_flyout_keeps_open_on_the_hosts_layers_and_leaves_escape_to_it() {
    let mut folded = layout();
    folded.set_folded("left", true);
    let mut harness = hosted(folded, Dismiss::OUTSIDE_PRESS, Dismiss::ANY);
    harness.state_mut().app.flyout = Some(Flyout::new("files", "left"));
    harness.run();
    assert!(harness.query_by_label("Flyout files").is_some());

    // Escape is the host's.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().app.flyout.is_some());

    // A press on the host's overlay is inside.
    hosted_click(&mut harness, pos2(650.0, 40.0));
    assert!(harness.state().app.flyout.is_some());

    // Elsewhere outside, it closes.
    hosted_click(&mut harness, pos2(500.0, 300.0));
    assert!(harness.state().app.flyout.is_none());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn by_default_a_press_on_another_layer_still_closes_the_flyout() {
    let mut folded = layout();
    folded.set_folded("left", true);
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            egui::Area::new(egui::Id::unique(OVERLAY))
                .order(egui::Order::Foreground)
                .fixed_pos(pos2(600.0, 20.0))
                .show(ui.ctx(), |ui| {
                    ui.allocate_exact_size(vec2(150.0, 60.0), egui::Sense::click());
                });
            let laid = LayoutArea::new(&mut app.layout)
                .flyout(app.flyout.clone())
                .show_inside(ui, &mut Viewer);
            if laid
                .events
                .iter()
                .any(|e| matches!(e, LayoutEvent::FlyoutClosed { .. }))
            {
                app.flyout = None;
            }
        },
        App {
            layout: folded,
            flyout: Some(Flyout::new("files", "left")),
            events: Vec::new(),
        },
    );
    harness.run();
    assert!(harness.state().flyout.is_some());
    click(&mut harness, pos2(650.0, 40.0));
    assert!(harness.state().flyout.is_none());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_zone_lying_over_resizes_from_its_free_edge() {
    let mut over = layout();
    over.set_over("left", true);
    let mut harness = harness(over);
    let edge = harness.get_by_label("Resize left");
    assert_eq!(edge.accesskit_node().role(), Role::Splitter);
    let from = edge.rect().center();
    drag_by(&mut harness, from, 60.0);
    assert_eq!(state(&harness), (false, true));
    let size = harness.state().layout.zone("left").unwrap().state.size;
    assert_eq!(size, Some(260.0));
    assert!(
        harness
            .state()
            .events
            .iter()
            .any(|e| matches!(e, LayoutEvent::Resized { size, .. } if *size == 260.0))
    );
    // Not under its least size.
    let from = harness.get_by_label("Resize left").rect().center();
    drag_by(&mut harness, from, -250.0);
    let size = harness.state().layout.zone("left").unwrap().state.size;
    assert_eq!(size, Some(100.0));
}
