//! The floats of both pages: a tab dragged out of the One dock page's dock, and the Dock space
//! page's Float group, open a floating window drawn by `FloatsArea`, whose top tab bar ends
//! with the window's move handle.

use egui::accesskit::Role;
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack_demo::{DemoApp, Page};

fn harness() -> Harness<'static, DemoApp> {
    harness_on(Page::Space)
}

fn harness_on(page: Page) -> Harness<'static, DemoApp> {
    let mut app = DemoApp::default();
    app.page = page;
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

#[test]
fn a_tab_dragged_out_of_the_first_page_becomes_a_float_with_a_move_handle() {
    let mut harness = harness_on(Page::Dock);
    assert!(harness.query_by_label("Move window").is_none());

    // Out of the dock: released over the toolbar, above it.
    let preview = harness
        .get_by_role_and_label(Role::Tab, "Preview")
        .rect()
        .center();
    let toolbar = harness.get_by_label("One dock").rect();
    let target = egui::pos2(preview.x - 150.0, toolbar.center().y);
    harness.hover_at(preview);
    harness.step();
    harness.drag_at(preview);
    harness.step();
    for step in 1..=10 {
        harness.hover_at(preview.lerp(target, step as f32 / 10.0));
        harness.step();
    }
    harness.drop_at(target);
    harness.run();

    let app = harness.state();
    assert_eq!(app.dock_floats.floats.len(), 1, "one float");
    assert!(
        app.dock
            .state()
            .main_surface()
            .find_tab(&"Preview".to_owned())
            .is_none(),
        "the tab left the dock"
    );
    assert!(
        app.dock
            .state()
            .iter_surfaces()
            .all(|surface| !matches!(surface, egui_stack::Surface::Window(..))),
        "no legacy window"
    );
    assert!(
        harness.query_by_label("Move window").is_some(),
        "the float's top tab bar has the window's move handle"
    );
}

#[test]
fn new_float_opens_a_tab_in_a_float_on_both_pages() {
    for page in [Page::Dock, Page::Space] {
        let mut harness = harness_on(page);
        harness.get_by_label("New float").click();
        harness.run();
        let floats = match page {
            Page::Dock => harness.state().dock_floats.floats.len(),
            Page::Space => harness.state().floats.floats.len(),
        };
        assert_eq!(floats, 1, "{page:?}: one float");
        assert!(
            harness.query_by_label("Move window").is_some(),
            "{page:?}: the float has the window's move handle"
        );
    }
}
