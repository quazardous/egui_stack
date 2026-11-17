//! A zone the solver folds by itself (`Constraints::auto_fold`) or brings back is reported in
//! `Laid::events`, once per change: `LayoutEvent::AutoFolded` and `AutoUnfolded`. Its
//! `ZoneState::folded` is not touched. `Layout::set_folded`, the application's own act,
//! reports nothing.

use egui::{Ui, vec2};
use egui_kittest::Harness;
use egui_stack::layout::{
    Content, Dir, Fold, Layout, LayoutArea, LayoutEvent, Priority, Size, Zone, ZoneName, ZoneViewer,
};

struct Viewer;

impl ZoneViewer for Viewer {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        ui.label(zone.to_string());
    }
}

struct App {
    layout: Layout,
    events: Vec<LayoutEvent>,
}

/// `left`, `centre` (growing, 300 points at least), `right` (low priority), both sides
/// folding by themselves when short of room.
fn harness() -> Harness<'static, App> {
    let side = |name: &str, priority| {
        Zone::leaf(name, Content::App).with(|c| {
            c.default = Some(Size::Points(150.0));
            c.min = Some(Size::Points(60.0));
            c.fold = Fold::ToStrip;
            c.auto_fold = true;
            c.priority = priority;
        })
    };
    let layout = Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            side("left", Priority::default()),
            Zone::leaf("centre", Content::App).with(|c| {
                c.grow = true;
                c.min = Some(Size::Points(300.0));
            }),
            side("right", Priority::Low),
        ],
    ));
    let mut harness = Harness::builder().with_size([900.0, 300.0]).build_ui_state(
        |ui, app: &mut App| {
            let laid = LayoutArea::new(&mut app.layout).show_inside(ui, &mut Viewer);
            app.events.extend(laid.events);
        },
        App {
            layout,
            events: Vec::new(),
        },
    );
    harness.run();
    harness
}

fn auto_events(app: &mut App) -> Vec<LayoutEvent> {
    std::mem::take(&mut app.events)
        .into_iter()
        .filter(|event| {
            matches!(
                event,
                LayoutEvent::AutoFolded { .. } | LayoutEvent::AutoUnfolded { .. }
            )
        })
        .collect()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_zone_folded_by_the_solver_is_reported_once_and_once_back() {
    let mut harness = harness();
    assert!(auto_events(harness.state_mut()).is_empty());

    // Short of room: the low-priority side folds by itself, said once over several frames.
    harness.set_size(vec2(410.0, 300.0));
    harness.run_steps(4);
    assert_eq!(
        auto_events(harness.state_mut()),
        [LayoutEvent::AutoFolded {
            zone: "right".into()
        }]
    );
    assert!(!harness.state().layout.zone("right").unwrap().state.folded);

    // The room back: it comes back, said once.
    harness.set_size(vec2(900.0, 300.0));
    harness.run_steps(4);
    assert_eq!(
        auto_events(harness.state_mut()),
        [LayoutEvent::AutoUnfolded {
            zone: "right".into()
        }]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn folding_from_the_application_reports_nothing() {
    let mut harness = harness();
    harness.state_mut().layout.set_folded("left", true);
    harness.run_steps(3);
    harness.state_mut().layout.set_folded("left", false);
    harness.run_steps(3);
    assert!(
        harness.state().events.is_empty(),
        "{:?}",
        harness.state().events
    );
}
