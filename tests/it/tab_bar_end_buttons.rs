//! The buttons at the right end of a tab bar (the group grip, the close-all button, and the
//! add button once it sits at the end) are packed together: no empty slot between them, with
//! the add button right after the last tab (the default), at the end (`TabAddAlign::Right`),
//! or in its slot once the tabs fill the bar.

use egui::{Id, Rect, Ui, WidgetText, accesskit::Role, pos2};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::{DockArea, DockState, Style, TabAddAlign, TabViewer};

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
        ui.label(tab.as_str());
    }
}

struct App {
    dock_state: DockState<String>,
    align: TabAddAlign,
}

fn harness(tabs: usize, align: TabAddAlign) -> Harness<'static, App> {
    let names = (0..tabs).map(|i| format!("Tab number {i}")).collect();
    let app = App {
        dock_state: DockState::new(names),
        align,
    };
    let mut harness = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        |ui, app: &mut App| {
            let mut style = Style::from_egui(ui.style());
            style.buttons.add_tab_align = app.align;
            DockArea::new(&mut app.dock_state)
                .style(style)
                .show_add_buttons(true)
                .show_inside(ui, &mut Viewer);
        },
        app,
    );
    harness.run();
    harness
}

fn rect_of(harness: &Harness<'_, App>, role: Role, label: &str) -> Rect {
    let node = harness.get_by_role_and_label(role, label);
    let bounds = node.accesskit_node().bounding_box().expect("bounds");
    Rect::from_min_max(
        pos2(bounds.x0 as f32, bounds.y0 as f32),
        pos2(bounds.x1 as f32, bounds.y1 as f32),
    )
}

/// The widest gap allowed between two buttons side by side: none, give or take rounding.
const SPACING: f32 = 1.0;

/// Checks that `buttons`, sorted left to right, touch each other.
fn packed(what: &str, mut buttons: Vec<(&str, Rect)>) {
    buttons.sort_by(|a, b| a.1.left().total_cmp(&b.1.left()));
    for pair in buttons.windows(2) {
        let ((left_name, left), (right_name, right)) = (pair[0], pair[1]);
        let gap = right.left() - left.right();
        assert!(
            (-SPACING..=SPACING).contains(&gap),
            "{what}: a gap of {gap} points between {left_name} ({left:?}) and {right_name} \
             ({right:?})"
        );
    }
}

fn check(tabs: usize, align: TabAddAlign, plus_at_end: bool) {
    let harness = harness(tabs, align);
    let grip = rect_of(&harness, Role::Button, "Move group");
    let close = rect_of(&harness, Role::Button, "Close all tabs");
    let plus = rect_of(&harness, Role::Button, "Add tab");
    let mut buttons = vec![("the grip", grip), ("the close-all button", close)];
    if plus_at_end {
        buttons.push(("the add button", plus));
    } else {
        assert!(
            plus.right() < grip.left() - 100.0,
            "{tabs} tabs, {align:?}: the add button follows the tabs, away from the end: \
             {plus:?}, grip {grip:?}"
        );
    }
    packed(&format!("{tabs} tabs, {align:?}"), buttons);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn no_gap_with_the_add_button_after_the_tabs() {
    check(2, TabAddAlign::Left, false);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn no_gap_with_the_add_button_at_the_end() {
    check(2, TabAddAlign::Right, true);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn no_gap_once_the_tabs_fill_the_bar() {
    check(30, TabAddAlign::Left, true);
    check(30, TabAddAlign::Right, true);
}
