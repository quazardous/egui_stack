//! The Dock space page's floats: the toolbar's Float group moves the focused group into a
//! floating window, whose top tab bar ends with the window's move handle.

use egui::accesskit::Role;
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

#[test]
fn float_group_opens_a_float_with_a_move_handle() {
    let mut harness = harness();
    assert!(
        harness.query_by_label("Move window").is_none(),
        "no move handle outside floats"
    );

    harness.get_by_role_and_label(Role::Tab, "Editor").click();
    harness.run();
    harness.get_by_label("Float group").click();
    harness.run();

    assert_eq!(harness.state().floats.floats.len(), 1, "one float");
    assert!(
        harness.query_by_label("Move window").is_some(),
        "the float's top tab bar has the window's move handle"
    );
}

#[test]
fn a_dragged_tab_previews_its_content_in_its_ghost() {
    let mut harness = harness();
    assert!(harness.query_by_label("main.rs").is_none());
    harness.state_mut().tabs.ghost_preview = true;
    harness.run();
    let editor = harness
        .get_by_role_and_label(Role::Tab, "Editor")
        .rect()
        .center();
    let target = editor + egui::vec2(-200.0, 300.0);
    harness.hover_at(editor);
    harness.step();
    harness.drag_at(editor);
    harness.step();
    for step in 1..=10 {
        harness.hover_at(editor.lerp(target, step as f32 / 10.0));
        harness.step();
    }
    // The editor's content, its file name above the code, is drawn in the ghost only: the
    // editor is not the active tab of its group.
    assert!(
        harness.query_by_label("main.rs").is_some(),
        "the ghost previews the editor"
    );
    harness.drop_at(target);
    harness.run();
}
