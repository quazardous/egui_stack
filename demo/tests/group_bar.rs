//! The demo's toggles for a tab group's bar and its floats: "Eject group buttons" adds the
//! Eject group button to each tab bar, "Move floats from anywhere" moves a float from the empty
//! part of its tab bar; a right click on that empty part opens the group's menu.

use egui::{Event, Modifiers, PointerButton, Pos2, Rect, accesskit::Role, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack_demo::{DemoApp, Page};

fn harness() -> Harness<'static, DemoApp> {
    let mut app = DemoApp::default();
    app.page = Page::Space;
    let mut harness = Harness::builder()
        .with_size([1100.0, 700.0])
        .build_ui_state(|ui, app: &mut DemoApp| app.show(ui), app);
    harness.run();
    harness
}

/// Floats the Editor's group by the Float group button.
fn float_editor(harness: &mut Harness<'static, DemoApp>) {
    harness.get_by_role_and_label(Role::Tab, "Editor").click();
    harness.run();
    harness.get_by_label("Float group").click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().floats.floats.len(), 1, "one float");
}

/// A point of the float's top tab bar left of its group grip: the empty part of the bar.
fn float_bar_gap(harness: &Harness<'static, DemoApp>) -> Pos2 {
    let handle = harness.get_by_label("Move window").rect();
    let grip = harness
        .query_all_by_role_and_label(Role::Button, "Move group")
        .map(|node| node.rect())
        .find(|rect: &Rect| (rect.center().y - handle.center().y).abs() < 1.0)
        .expect("the float's bar has a group grip");
    pos2(grip.left() - 3.0, grip.center().y)
}

#[test]
fn the_eject_toggle_adds_the_eject_group_button() {
    let mut harness = harness();
    assert!(
        harness.query_by_label("Eject group").is_none(),
        "off by default"
    );
    harness
        .get_by_role_and_label(Role::Button, "Eject group buttons")
        .click();
    harness.run();
    harness.run();
    let ejects = harness.query_all_by_label("Eject group").count();
    assert!(ejects >= 2, "a button per tab bar: {ejects}");
    harness
        .get_all_by_label("Eject group")
        .next()
        .unwrap()
        .click();
    harness.run();
    harness.run();
    assert_eq!(harness.state().floats.floats.len(), 1, "the group floats");
}

#[test]
fn moving_floats_from_anywhere_moves_a_float_by_its_bar_gap() {
    let mut harness = harness();
    harness
        .get_by_role_and_label(Role::Button, "Move floats from anywhere")
        .click();
    harness.run();
    float_editor(&mut harness);
    let before = harness.state().floats.floats[0].place.rect;
    let from = float_bar_gap(&harness);
    let to = from + vec2(-80.0, 60.0);
    harness.hover_at(from);
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    for step in 1..=10 {
        harness.hover_at(from + (to - from) * (step as f32 / 10.0));
        harness.run();
    }
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    harness.run();
    let after = harness.state().floats.floats[0].place.rect;
    assert!(
        ((after.min - before.min) - (to - from)).length() < 2.0,
        "the float follows: {before:?} → {after:?}"
    );
}

#[test]
fn a_right_click_on_a_float_bar_gap_opens_the_group_s_menu() {
    let mut harness = harness();
    float_editor(&mut harness);
    let at = float_bar_gap(&harness);
    harness.hover_at(at);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: at,
            button: PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, "Add tab")
            .any(|node| node.rect().width() > 30.0),
        "the group's menu has Add tab"
    );
}
