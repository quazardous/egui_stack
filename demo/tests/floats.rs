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

    // One close button on that bar, the float's, right of the handle: the group's close all
    // button would close the window too.
    let handle = harness.get_by_label("Move window").rect();
    let on_bar = |rect: egui::Rect| (rect.center().y - handle.center().y).abs() < 1.0;
    let closes: Vec<_> = ["Close window", "Close all tabs"]
        .into_iter()
        .flat_map(|label| harness.query_all_by_label(label).map(|node| node.rect()))
        .filter(|rect| on_bar(*rect))
        .collect();
    assert_eq!(
        closes.len(),
        1,
        "one close button on the float's bar: {closes:?}"
    );
    assert!(
        closes[0].left() >= handle.right() - 1.0,
        "the float's close button, right of its move handle"
    );
}

/// A float's tab bar keeps the add button its group had in the dock, on both pages, and a
/// click on it opens a new tab in that float's group.
#[test]
fn a_float_shows_an_add_button_that_adds_a_tab_to_its_group() {
    for page in [Page::Dock, Page::Space] {
        let mut harness = harness_on(page);
        harness.get_by_label("New float").click();
        harness.run();
        let handle = harness.get_by_label("Move window").rect();
        let on_bar = |rect: egui::Rect| (rect.center().y - handle.center().y).abs() < 1.0;
        let pluses: Vec<_> = harness
            .query_all_by_label("Add tab")
            .map(|node| node.rect())
            .filter(|rect| on_bar(*rect))
            .collect();
        assert_eq!(
            pluses.len(),
            1,
            "{page:?}: one add button on the float's bar: {pluses:?}"
        );
        assert!(
            pluses[0].right() <= handle.left() + 1.0,
            "{page:?}: the add button is left of the move handle"
        );

        let float_tabs = |harness: &Harness<'static, DemoApp>| {
            let app = harness.state();
            let (layout, space) = match page {
                Page::Dock => (&app.dock_floats, &app.dock),
                Page::Space => (&app.floats, &app.space),
            };
            let float = layout.floats.first().expect("one float");
            let dock = float.dock_name().expect("the float shows a dock");
            let surface = space.dock_surface(dock).expect("its dock has a surface");
            space.state()[surface]
                .iter()
                .filter_map(|node| node.get_leaf())
                .map(|leaf| leaf.tabs.len())
                .sum::<usize>()
        };
        let before = float_tabs(&harness);
        harness
            .query_all_by_label("Add tab")
            .find(|node| on_bar(node.rect()))
            .expect("the float's add button")
            .click();
        harness.run();
        assert_eq!(
            float_tabs(&harness),
            before + 1,
            "{page:?}: the add button adds a tab to the float's group"
        );
    }
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

/// A float resizes from every edge and corner, its left and top ones included: the dragged
/// sides follow the pointer, the others stay.
#[test]
fn a_float_resizes_from_its_left_and_top_edges_too() {
    let float_rect =
        |harness: &Harness<'static, DemoApp>| harness.state().dock_floats.floats[0].place.rect;
    for (gx, gy) in [
        (-1.0_f32, 0.0_f32),
        (0.0, -1.0),
        (-1.0, -1.0),
        (1.0, 0.0),
        (0.0, 1.0),
        (1.0, 1.0),
    ] {
        let mut harness = harness_on(Page::Dock);
        harness.get_by_label("New float").click();
        harness.run();
        let before = float_rect(&harness);
        let at = |sign: f32, min: f32, max: f32| match sign {
            s if s < 0.0 => min + 1.0,
            s if s > 0.0 => max - 1.0,
            _ => (min + max) / 2.0,
        };
        let from = egui::pos2(
            at(gx, before.left(), before.right()),
            at(gy, before.top(), before.bottom()),
        );
        let delta = egui::vec2(gx, gy) * 30.0;
        harness.hover_at(from);
        harness.step();
        harness.drag_at(from);
        harness.step();
        for step in 1..=10 {
            harness.hover_at(from + delta * (step as f32 / 10.0));
            harness.step();
        }
        harness.drop_at(from + delta);
        harness.run();
        let after = float_rect(&harness);
        let what = format!("grab ({gx}, {gy}): {before:?} -> {after:?}");
        let near = |a: f32, b: f32| (a - b).abs() <= 2.0;
        assert!(
            near(after.left(), before.left() + delta.x.min(0.0)),
            "left: {what}"
        );
        assert!(
            near(after.right(), before.right() + delta.x.max(0.0)),
            "right: {what}"
        );
        assert!(
            near(after.top(), before.top() + delta.y.min(0.0)),
            "top: {what}"
        );
        assert!(
            near(after.bottom(), before.bottom() + delta.y.max(0.0)),
            "bottom: {what}"
        );
    }
}
