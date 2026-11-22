//! A tab starts being dragged once the pointer is further than
//! [`OverlayFeel::drag_activation_distance`](egui_stack::OverlayFeel::drag_activation_distance)
//! from where the button was pressed: by default as soon as egui stops taking the gesture for a
//! click, in every direction, so there is no distance at which a tab neither clicks nor drags.

use egui::{Id, Ui, Vec2, WidgetText, accesskit::Role, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, DraggedTab, Style, TabViewer};

struct Viewer;

impl TabViewer for Viewer {
    type Tab = String;

    fn id(&mut self, tab: &mut String) -> Id {
        Id::unique(tab.as_str())
    }

    fn title(&mut self, tab: &mut String) -> WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut String) {
        ui.label(format!("Body of {tab}"));
    }
}

struct App {
    dock_state: DockState<String>,
    activation: Option<Vec2>,
    seen: Option<DraggedTab>,
}

fn harness(pixels_per_point: f32, activation: Option<Vec2>) -> Harness<'static, App> {
    let app = App {
        dock_state: DockState::new(vec!["First".to_owned(), "Second".to_owned()]),
        activation,
        seen: None,
    };
    let mut harness = Harness::builder()
        .with_size([600.0, 400.0])
        .with_pixels_per_point(pixels_per_point)
        // 60 frames a second: the press stays a click by its duration while the pointer moves.
        .with_step_dt(1.0 / 60.0)
        .build_ui_state(
            |ui, app: &mut App| {
                // Room above the dock, so a pointer moving up stays on the screen.
                ui.add_space(100.0);
                let id = Id::unique("dock");
                let mut style = Style::from_egui(ui.style());
                if let Some(activation) = app.activation {
                    style.overlay.feel.drag_activation_distance = activation;
                }
                DockArea::new(&mut app.dock_state)
                    .id(id)
                    .style(style)
                    .show_inside(ui, &mut Viewer);
                app.seen = DockArea::dragged_tab(ui.ctx(), id);
            },
            app,
        );
    harness.run();
    harness
}

/// Presses on the tab `Second`, then moves the pointer one point a frame along `direction`;
/// returns the distance from the press at which the tab became dragged.
fn activation_distance(pixels_per_point: f32, activation: Option<Vec2>, direction: Vec2) -> u32 {
    let mut harness = harness(pixels_per_point, activation);
    let start = harness
        .get_by_role_and_label(Role::Tab, "Second")
        .rect()
        .center();
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for distance in 1..=60 {
        harness.hover_at(start + direction * distance as f32);
        harness.step();
        if harness.state().seen.is_some() {
            return distance;
        }
    }
    panic!("no drag within 60 points along {direction:?} at {pixels_per_point}x");
}

const DIRECTIONS: [(&str, Vec2); 4] = [
    ("right", vec2(1.0, 0.0)),
    ("left", vec2(-1.0, 0.0)),
    ("down", vec2(0.0, 1.0)),
    ("up", vec2(0.0, -1.0)),
];

const SCALES: [f32; 4] = [1.0, 1.25, 1.5, 2.0];

#[test]
fn the_default_matches_eguis_click_distance() {
    let style = Style::from_egui(&egui::Style::default());
    let click = egui::InputOptions::default().max_click_dist;
    assert_eq!(
        style.overlay.feel.drag_activation_distance,
        Vec2::splat(click)
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn by_default_a_tab_drags_as_soon_as_it_stops_being_a_click() {
    // egui's click distance is 6 points: one point further, the press is a drag, and the tab
    // with it.
    for scale in SCALES {
        for (name, direction) in DIRECTIONS {
            assert_eq!(
                activation_distance(scale, None, direction),
                7,
                "{name} at {scale}x"
            );
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_custom_distance_is_measured_from_the_press() {
    let custom = Some(vec2(20.0, 10.0));
    for scale in SCALES {
        for (name, direction) in DIRECTIONS {
            let expected = if direction.x != 0.0 { 21 } else { 11 };
            assert_eq!(
                activation_distance(scale, custom, direction),
                expected,
                "{name} at {scale}x"
            );
        }
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_distance_below_the_click_distance_still_waits_for_egui() {
    // Under egui's click distance, the press is still a click: the tab drags once it is not.
    let custom = Some(Vec2::splat(2.0));
    for (name, direction) in DIRECTIONS {
        assert_eq!(activation_distance(1.0, custom, direction), 7, "{name}");
    }
}
