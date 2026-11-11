//! A host's overlay stays above a zone lying over its neighbour when the
//! host pins the zone under it (`LayoutArea::sublayer_of`), even once the
//! zone is clicked; without it, egui raises whatever was clicked last.

use egui::{Id, LayerId, Order, Pos2, Ui, pos2, vec2};
use egui_kittest::Harness;
use egui_stack::layout::{
    Content, Dir, Dismiss, Flyout, Fold, Layout, LayoutArea, Size, Zone, ZoneName, ZoneViewer,
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

/// A 200-point side lying over the centre, next to its strip.
fn layout() -> Layout {
    let mut layout = Layout::new(Zone::split(
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
    ));
    layout.set_over("left", true);
    layout
}

struct Host {
    layout: Layout,
    pinned: bool,
}

/// The host's base layer for the zones lying over; it never moves, so it
/// stays under everything shown after it in the foreground.
fn base() -> LayerId {
    LayerId::new(Order::Foreground, Id::unique("host.base"))
}

fn overlay() -> LayerId {
    LayerId::new(Order::Foreground, Id::unique("host.card"))
}

fn harness(pinned: bool) -> Harness<'static, Host> {
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, host: &mut Host| {
            // The base goes into egui's order on the first pass, before any
            // other foreground layer (a placeholder child keeps it there).
            ui.ctx().set_sublayer(
                base(),
                LayerId::new(Order::Foreground, Id::unique("host.ph")),
            );
            // The host's card, at the window's bottom left, over where the
            // zone lies.
            egui::Area::new(overlay().id)
                .order(Order::Foreground)
                .fixed_pos(pos2(40.0, 300.0))
                .show(ui.ctx(), |ui| {
                    ui.allocate_exact_size(vec2(150.0, 60.0), egui::Sense::click());
                });
            let mut area = LayoutArea::new(&mut host.layout).over_dismiss(Dismiss::NEVER);
            if host.pinned {
                area = area.sublayer_of(base());
            }
            area.show_inside(ui, &mut Viewer);
        },
        Host {
            layout: layout(),
            pinned,
        },
    );
    harness.run();
    harness
}

fn click(harness: &mut Harness<'_, Host>, pos: Pos2) {
    harness.hover_at(pos);
    harness.step();
    harness.drag_at(pos);
    harness.step();
    harness.drop_at(pos);
    harness.run();
}

/// The layer on top at a point both the card and the zone cover.
fn top(harness: &Harness<'_, Host>) -> Option<LayerId> {
    harness.ctx.layer_id_at(pos2(100.0, 330.0))
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_zone_lying_over_stays_under_the_hosts_overlay_when_clicked() {
    let mut harness = harness(true);
    assert_eq!(top(&harness), Some(overlay()));
    // A click on the zone, away from the card, keeps it lying over...
    click(&mut harness, pos2(100.0, 100.0));
    assert!(harness.state().layout.zone("left").unwrap().state.over);
    // ...and under the card.
    assert_eq!(top(&harness), Some(overlay()));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn without_a_parent_the_clicked_zone_covers_the_overlay() {
    let mut harness = harness(false);
    click(&mut harness, pos2(100.0, 100.0));
    assert_ne!(top(&harness), Some(overlay()));
}
