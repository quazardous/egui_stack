//! The inspection picker, headless: its shortcut, what it picks under the pointer (a tab, a
//! body, a separator), the pin on a click, and the text Copy puts on the clipboard.

use egui::{Key, OutputCommand, Pos2, pos2};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{NodeIndex, TabIndex};
use egui_stack_demo::{
    DemoApp,
    picker::{PartKind, SHORTCUT},
};

fn harness() -> Harness<'static, DemoApp> {
    let mut harness = Harness::builder()
        .with_size([1100.0, 700.0])
        .build_ui_state(|ui, app: &mut DemoApp| app.show(ui), DemoApp::default());
    harness.run();
    harness
}

/// Enters the picking mode with its shortcut.
fn start_picking(harness: &mut Harness<'_, DemoApp>) {
    harness.key_press_modifiers(SHORTCUT.modifiers, SHORTCUT.logical_key);
    harness.run();
    assert!(
        harness.state().picker.picking,
        "the shortcut enters the mode"
    );
    assert!(harness.state().picker.open, "the mode opens the panel");
}

fn hover(harness: &mut Harness<'_, DemoApp>, pos: Pos2) {
    harness.hover_at(pos);
    harness.run();
}

#[test]
fn picks_a_tab_and_pins_it_on_a_click() {
    let mut harness = harness();
    start_picking(&mut harness);
    let console = harness.get_by_label("Console").rect().center();
    hover(&mut harness, console);

    let pick = harness.state().picker.hovered.clone().expect("a pick");
    assert_eq!(pick.kind, PartKind::Tab);
    assert_eq!(pick.tab, Some(TabIndex(1)), "Console is the second tab");
    let tree = harness.state().dock_state.main_surface();
    assert_eq!(
        pick.node,
        tree.find_tab(&"Console".to_owned()).map(|(node, _)| node)
    );
    assert!(pick.rect.contains(console));
    assert!(pick.text.contains("tab \"Console\""), "{}", pick.text);
    assert!(pick.text.contains("tab.inactive"), "{}", pick.text);

    harness.drag_at(console);
    harness.drop_at(console);
    harness.run();
    let picker = &harness.state().picker;
    assert!(!picker.picking, "a click leaves the mode");
    assert_eq!(picker.pinned.as_ref().map(|p| p.kind), Some(PartKind::Tab));
    assert!(
        harness.state().dock_state.focused_leaf().is_none(),
        "the click went to the picker, not the dock"
    );

    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().picker.pinned.is_some(), "the pin stays");
}

#[test]
fn picks_a_body() {
    let mut harness = harness();
    start_picking(&mut harness);
    let tree = harness.state().dock_state.main_surface();
    let (node, _) = tree
        .find_tab(&"Preview".to_owned())
        .expect("Preview is docked");
    let viewport = tree[node].get_leaf().expect("a leaf").viewport;
    hover(&mut harness, viewport.center());

    let pick = harness.state().picker.hovered.clone().expect("a pick");
    assert_eq!(pick.kind, PartKind::Body);
    assert_eq!(pick.node, Some(node));
    assert_eq!(pick.rect, viewport);
    assert!(
        pick.text.contains("Drop zone here: Center"),
        "{}",
        pick.text
    );
    assert!(pick.text.contains("tab.tab_body"), "{}", pick.text);

    // Near the left edge, the band that splits the leaf.
    hover(
        &mut harness,
        pos2(
            viewport.left() + viewport.width() * 0.1,
            viewport.center().y,
        ),
    );
    let pick = harness.state().picker.hovered.clone().expect("a pick");
    assert!(pick.text.contains("Drop zone here: Left"), "{}", pick.text);

    harness.key_press(Key::Escape);
    harness.run();
    let picker = &harness.state().picker;
    assert!(!picker.picking && picker.hovered.is_none(), "Esc leaves");
}

#[test]
fn picks_a_separator() {
    let mut harness = harness();
    start_picking(&mut harness);
    let root = &harness.state().dock_state.main_surface()[NodeIndex::root()];
    let linear = root.get_linear().expect("the root is a row");
    let at = pos2(
        linear.rect.left() + linear.rect.width() * linear.boundary(0),
        linear.rect.center().y,
    );
    hover(&mut harness, at);

    let pick = harness.state().picker.hovered.clone().expect("a pick");
    assert_eq!(pick.kind, PartKind::Separator);
    assert_eq!(pick.node, Some(NodeIndex::root()));
    assert!(
        pick.text.contains("separator, after child 0 of the row"),
        "{}",
        pick.text
    );
    assert!(pick.text.contains("separator: width"), "{}", pick.text);
}

#[test]
fn copy_puts_the_pick_on_the_clipboard() {
    let mut harness = harness();
    start_picking(&mut harness);
    let guide = harness.get_by_label("Guide").rect().center();
    harness.drag_at(guide);
    harness.drop_at(guide);
    harness.run();
    let text = harness
        .state()
        .picker
        .pinned
        .as_ref()
        .expect("a pin")
        .text
        .clone();

    harness.get_by_label("Copy").click();
    harness.step();
    let copied: Vec<_> = harness
        .output()
        .platform_output
        .commands
        .iter()
        .filter_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(copied, std::slice::from_ref(&text));
    assert!(text.starts_with("Part: tab \"Guide\""), "{text}");
    assert!(text.contains("Surface: SurfaceIndex(0), main"), "{text}");
}
