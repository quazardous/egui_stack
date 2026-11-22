//! `TabStyle::padding` sets the space on each side of a tab's title, so tabs can be narrower.

use egui::{Id, Ui, WidgetText, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{DockArea, DockState, Style, TabViewer};

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

/// The width of the tab `Title` with this padding (and no close button).
fn tab_width(padding: Option<f32>) -> f32 {
    let mut harness = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        move |ui, dock_state: &mut DockState<String>| {
            let mut style = Style::from_egui(ui.style());
            if let Some(padding) = padding {
                style.tab.padding = padding;
            }
            DockArea::new(dock_state)
                .style(style)
                .show_close_buttons(false)
                .show_inside(ui, &mut Viewer);
        },
        DockState::new(vec!["Title".to_owned()]),
    );
    harness.run();
    harness
        .get_by_role_and_label(Role::Tab, "Title")
        .rect()
        .width()
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn padding_sets_the_space_around_the_title() {
    let default = tab_width(None);
    let unpadded = tab_width(Some(0.0));
    let wide = tab_width(Some(20.0));

    assert_eq!(Style::default().tab.padding, 8.0);
    assert!(
        (default - unpadded - 16.0).abs() < 0.01,
        "{default} {unpadded}"
    );
    assert!((wide - unpadded - 40.0).abs() < 0.01, "{wide} {unpadded}");
    // A negative padding counts as none.
    assert!((tab_width(Some(-5.0)) - unpadded).abs() < 0.01);
}
