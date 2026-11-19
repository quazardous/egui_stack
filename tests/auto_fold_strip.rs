//! A zone the solver folded short of room (`Shown::AutoFolded`) keeps a working strip: a click
//! opens it over its neighbour, and closing it puts it back in its automatic state, so it
//! unfolds by itself once the room comes back. `ZoneViewer::on_strip` can override what a
//! strip click does, and every click is reported (`LayoutEvent::StripClicked`, and
//! `stack.layout.changed` with the `bus` feature).

use egui::{Key, Response, Ui, accesskit::Role, vec2};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::layout::{
    Content, Dir, Fold, Layout, LayoutArea, LayoutEvent, Shown, Size, StripAction, Zone, ZoneName,
    ZoneViewer,
};

struct Viewer {
    /// What `on_strip` returns.
    action: StripAction,
}

impl ZoneViewer for Viewer {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        ui.label(format!("Body of {zone}"));
    }

    fn on_strip(&mut self, _zone: &ZoneName, _shown: Shown, _response: &Response) -> StripAction {
        self.action
    }
}

struct App {
    layout: Layout,
    viewer: Viewer,
    events: Vec<LayoutEvent>,
    shown: Option<Shown>,
}

/// `left` (200 points, least 100, folding by itself) and a centre needing 300 points.
fn layout() -> Layout {
    Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::leaf("left", Content::App).with(|c| {
                c.min = Some(Size::Points(100.0));
                c.default = Some(Size::Points(200.0));
                c.fold = Fold::ToStrip;
                c.auto_fold = true;
            }),
            Zone::leaf("centre", Content::App).with(|c| {
                c.grow = true;
                c.min = Some(Size::Points(300.0));
            }),
        ],
    ))
}

/// Short of room for both: `left` is auto-folded.
fn harness(layout: Layout, action: StripAction) -> Harness<'static, App> {
    let mut harness = Harness::builder().with_size([360.0, 300.0]).build_ui_state(
        |ui, app: &mut App| {
            let laid = LayoutArea::new(&mut app.layout).show_inside(ui, &mut app.viewer);
            app.shown = laid.get("left").map(|zone| zone.shown);
            app.events.extend(laid.events);
        },
        App {
            layout,
            viewer: Viewer { action },
            events: Vec::new(),
            shown: None,
        },
    );
    harness.run();
    harness
}

/// `left`'s `(folded, over)` state.
fn state(harness: &Harness<'_, App>) -> (bool, bool) {
    let state = harness.state().layout.zone("left").unwrap().state;
    (state.folded, state.over)
}

fn shown(harness: &Harness<'_, App>) -> Shown {
    harness.state().shown.unwrap()
}

fn take_events(harness: &mut Harness<'_, App>) -> Vec<LayoutEvent> {
    std::mem::take(&mut harness.state_mut().events)
}

fn left() -> ZoneName {
    "left".into()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_click_on_an_auto_folded_strip_opens_the_zone_over_its_neighbour() {
    let mut harness = harness(layout(), StripAction::Default);
    assert_eq!(shown(&harness), Shown::AutoFolded);
    assert!(harness.query_by_label("Body of left").is_none());
    let grip = harness.get_by_label("Unfold left");
    assert_eq!(grip.accesskit_node().role(), Role::Button);
    assert!(!grip.accesskit_node().is_disabled());
    take_events(&mut harness);

    harness.get_by_label("Unfold left").click();
    harness.run();
    assert_eq!(shown(&harness), Shown::Over);
    assert_eq!(state(&harness), (false, true));
    assert!(
        harness
            .state()
            .layout
            .zone("left")
            .unwrap()
            .state
            .over_from_auto()
    );
    // Its body lies over the centre, which keeps its room.
    assert!(harness.query_by_label("Body of left").is_some());
    let body = harness.get_by_label("left").rect();
    assert!(body.width() >= 100.0, "{body:?}");
    assert_eq!(body.left(), 8.0 + 24.0);
    assert_eq!(harness.get_by_label("centre").rect().left(), 8.0 + 24.0);
    // The click and what it did; the zone still counts as auto-folded.
    assert_eq!(
        take_events(&mut harness),
        [
            LayoutEvent::StripClicked {
                zone: left(),
                shown: Shown::AutoFolded,
            },
            LayoutEvent::OpenedOver { zone: left() },
        ]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn closing_it_puts_it_back_in_its_automatic_state() {
    let mut harness = harness(layout(), StripAction::Default);

    // Escape closes it: back to auto-folded, not folded.
    harness.get_by_label("Unfold left").click();
    harness.run();
    take_events(&mut harness);
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(state(&harness), (false, false));
    assert_eq!(shown(&harness), Shown::AutoFolded);
    assert!(harness.query_by_label("Body of left").is_none());
    assert_eq!(
        take_events(&mut harness),
        [LayoutEvent::ClosedOver { zone: left() }]
    );

    // Its strip closes it the same way.
    harness.get_by_label("Unfold left").click();
    harness.run();
    take_events(&mut harness);
    harness.get_by_label("Fold left").click();
    harness.run();
    assert_eq!(state(&harness), (false, false));
    assert_eq!(shown(&harness), Shown::AutoFolded);
    assert_eq!(
        take_events(&mut harness),
        [
            LayoutEvent::StripClicked {
                zone: left(),
                shown: Shown::Over,
            },
            LayoutEvent::ClosedOver { zone: left() },
        ]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn once_closed_it_unfolds_by_itself_when_the_room_comes_back() {
    let mut harness = harness(layout(), StripAction::Default);
    harness.get_by_label("Unfold left").click();
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    take_events(&mut harness);

    harness.set_size(vec2(800.0, 300.0));
    harness.run_steps(3);
    assert_eq!(shown(&harness), Shown::Open);
    assert_eq!(harness.get_by_label("left").rect().width(), 200.0);
    assert_eq!(
        take_events(&mut harness),
        [LayoutEvent::AutoUnfolded { zone: left() }]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_hook_can_keep_the_strip_from_doing_anything() {
    let mut harness = harness(layout(), StripAction::Nothing);
    take_events(&mut harness);
    harness.get_by_label("Unfold left").click();
    harness.run();
    assert_eq!(shown(&harness), Shown::AutoFolded);
    assert_eq!(state(&harness), (false, false));
    // The click is still reported, for the application to act on.
    assert_eq!(
        take_events(&mut harness),
        [LayoutEvent::StripClicked {
            zone: left(),
            shown: Shown::AutoFolded,
        }]
    );

    // On a strip the user folded, nothing either.
    harness.set_size(vec2(800.0, 300.0));
    harness.state_mut().layout.set_folded("left", true);
    harness.run();
    take_events(&mut harness);
    harness.get_by_label("Unfold left").click();
    harness.run();
    assert_eq!(state(&harness), (true, false));
    assert_eq!(shown(&harness), Shown::Folded);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_hook_can_open_a_folded_zone_over_its_neighbour() {
    let mut folded = layout();
    folded.set_folded("left", true);
    let mut harness = harness(folded, StripAction::Over);
    harness.set_size(vec2(800.0, 300.0));
    harness.run();
    take_events(&mut harness);

    harness.get_by_label("Unfold left").click();
    harness.run();
    assert_eq!(shown(&harness), Shown::Over);
    assert!(
        !harness
            .state()
            .layout
            .zone("left")
            .unwrap()
            .state
            .over_from_auto()
    );
    assert_eq!(
        take_events(&mut harness),
        [
            LayoutEvent::StripClicked {
                zone: left(),
                shown: Shown::Folded,
            },
            LayoutEvent::OpenedOver { zone: left() },
        ]
    );
    // The user folded it: closed, it folds back.
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(state(&harness), (true, false));
    assert_eq!(
        take_events(&mut harness),
        [LayoutEvent::Folded { zone: left() }]
    );
}

#[cfg(feature = "bus")]
mod bus {
    use std::sync::{Arc, Mutex};

    use bus_kernel::Kernel;
    use egui_stack::DockSpace;
    use egui_stack::bus::bus_contract::{Bus, ListenOpts};
    use egui_stack::bus::{Change, LayoutChanged};

    use super::*;

    #[test]
    #[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
    fn the_click_goes_on_the_bus() {
        let bus = Arc::new(Kernel::new());
        let heard = Arc::new(Mutex::new(Vec::new()));
        let h = heard.clone();
        let mut space: DockSpace<String> =
            DockSpace::new().with_bus(bus.clone(), |tab: &String| tab.clone());
        let listen: &dyn Bus = &*bus;
        listen
            .listen::<LayoutChanged>("test.layout", ListenOpts::new(), move |change, _| {
                h.lock().unwrap().push((change.change, change.zone.clone()));
                Ok(())
            })
            .unwrap();

        let mut harness = harness(layout(), StripAction::Default);
        take_events(&mut harness);
        harness.get_by_label("Unfold left").click();
        harness.run();
        let mut laid = egui_stack::layout::Laid::default();
        laid.events = take_events(&mut harness);
        space.bind(&laid);
        let left = Some("left".to_owned());
        assert_eq!(
            heard.lock().unwrap().clone(),
            [
                (Change::StripClicked, left.clone()),
                (Change::OpenedOver, left)
            ]
        );
    }
}
