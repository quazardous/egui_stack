//! Where a tab bar's add button sits (`ButtonsStyle::add_tab_align`): right after the last
//! tab by default, at the end of the bar with `TabAddAlign::Right`, and in its slot at the end
//! once the tabs fill the bar.

use egui::{Id, Rect, Ui, WidgetText, accesskit::Role};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};
use egui_stack::{DockArea, DockState, Style, TabAddAlign, TabViewer};

#[derive(Default)]
struct Viewer {
    tab_rects: Vec<(String, Rect)>,
}

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

    fn on_tab_button(&mut self, tab: &mut String, response: &egui::Response) {
        self.tab_rects.push((tab.clone(), response.rect));
    }
}

struct App {
    dock_state: DockState<String>,
    viewer: Viewer,
    align: Option<TabAddAlign>,
}

fn harness(tabs: usize, align: Option<TabAddAlign>) -> Harness<'static, App> {
    let names = (0..tabs).map(|i| format!("Tab number {i}")).collect();
    let app = App {
        dock_state: DockState::new(names),
        viewer: Viewer::default(),
        align,
    };
    let mut harness = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        |ui, app: &mut App| {
            app.viewer.tab_rects.clear();
            let mut style = Style::from_egui(ui.style());
            if let Some(align) = app.align {
                style.buttons.add_tab_align = align;
            }
            DockArea::new(&mut app.dock_state)
                .style(style)
                .show_add_buttons(true)
                .show_inside(ui, &mut app.viewer);
        },
        app,
    );
    harness.run();
    harness
}

fn add_button(harness: &Harness<'_, App>) -> Rect {
    let node = harness.get_by_role_and_label(Role::Button, "Add tab");
    let bounds = node.accesskit_node().bounding_box().expect("bounds");
    Rect::from_min_max(
        egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
        egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
    )
}

fn last_tab_right(harness: &Harness<'_, App>) -> f32 {
    harness
        .state()
        .viewer
        .tab_rects
        .iter()
        .map(|(_, rect)| rect.right())
        .fold(f32::MIN, f32::max)
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_add_button_follows_the_last_tab_by_default() {
    assert_eq!(Style::default().buttons.add_tab_align, TabAddAlign::Left);
    let harness = harness(2, None);
    let plus = add_button(&harness);
    let last = last_tab_right(&harness);
    assert!(
        (plus.left() - last).abs() <= 2.0,
        "right after the last tab ({last}): {plus:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_add_button_sits_at_the_end_with_right() {
    let harness = harness(2, Some(TabAddAlign::Right));
    let plus = add_button(&harness);
    let last = last_tab_right(&harness);
    assert!(plus.left() > last + 100.0, "at the end ({last}): {plus:?}");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_full_bar_keeps_the_add_button_in_its_slot() {
    let left = add_button(&harness(30, None));
    let right = add_button(&harness(30, Some(TabAddAlign::Right)));
    assert_eq!(left, right);
}
