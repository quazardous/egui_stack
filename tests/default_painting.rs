//! The dock's default painting, shape by shape: tabs, bars, separators and the drop overlay.
//!
//! Each scene's shapes are compared with `tests/painting/<scene>.txt`. Run with
//! `UPDATE_SNAPSHOTS=1` to write the files again after an intended change.

use std::fmt::Write as _;

use egui::epaint::ClippedShape;

use egui::{Id, Pos2, Rect, Shape, Ui, WidgetText, pos2, vec2};
use egui_kittest::Harness;
use egui_stack::{
    DockArea, DockState, NodeIndex, OverlayType, Style, TabDestination, TabViewer,
    tab_viewer::DropVerdict,
};

struct Viewer {
    refuse: bool,
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
        ui.label(format!("Body of {tab}"));
    }

    fn on_tab_button(&mut self, tab: &mut String, response: &egui::Response) {
        self.tab_rects.push((tab.clone(), response.rect));
    }

    fn drop_allowed(&self, _tab: &String, _destination: &TabDestination) -> DropVerdict {
        if self.refuse {
            DropVerdict::Refuse(Some("Not here".into()))
        } else {
            DropVerdict::Allow
        }
    }
}

struct App {
    dock_state: DockState<String>,
    viewer: Viewer,
    overlay: OverlayType,
}

fn scene(refuse: bool, overlay: OverlayType) -> Harness<'static, App> {
    let mut dock_state = DockState::new(vec![
        "First".to_owned(),
        "Second".to_owned(),
        "Fourth".to_owned(),
    ]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    let app = App {
        dock_state,
        viewer: Viewer {
            refuse,
            tab_rects: Vec::new(),
        },
        overlay,
    };
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            app.viewer.tab_rects.clear();
            let mut style = Style::from_egui(ui.style());
            style.overlay.overlay_type = app.overlay;
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

fn tab_rect(harness: &Harness<'_, App>, title: &str) -> Rect {
    harness
        .state()
        .viewer
        .tab_rects
        .iter()
        .find(|(tab, _)| tab == title)
        .map(|(_, rect)| *rect)
        .unwrap()
}

/// The middle of the body of the leaf holding `Third`.
fn third_body(harness: &Harness<'_, App>) -> Pos2 {
    let leaf = harness.state().dock_state.main_surface()[NodeIndex::root().right()]
        .rect()
        .unwrap();
    pos2(leaf.center().x, leaf.center().y + 30.0)
}

/// Starts dragging `First` and holds it over `target`, without releasing it.
fn drag_first_to(harness: &mut Harness<'_, App>, target: impl Fn(&Harness<'_, App>) -> Pos2) {
    let start = tab_rect(harness, "First").center();
    let target = target(harness);
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=5 {
        harness.hover_at(start + vec2(0.0, 10.0 * step as f32));
        harness.step();
    }
    for _ in 0..5 {
        harness.hover_at(target);
        harness.step();
    }
}

fn shape_lines(out: &mut String, depth: usize, shape: &Shape) {
    let indent = "  ".repeat(depth);
    match shape {
        Shape::Vec(shapes) => {
            let _ = writeln!(out, "{indent}Vec");
            for shape in shapes {
                shape_lines(out, depth + 1, shape);
            }
        }
        // A galley's glyphs depend on the font atlas: its text, place and colours are enough.
        Shape::Text(text) => {
            let _ = writeln!(
                out,
                "{indent}Text {:?} at {:?} size {:?} fallback {:?} override {:?} opacity {:?}",
                text.galley.text(),
                text.pos,
                text.galley.size(),
                text.fallback_color,
                text.override_text_color,
                text.opacity_factor,
            );
        }
        shape => {
            let _ = writeln!(out, "{indent}{shape:?}");
        }
    }
}

fn digest(harness: &Harness<'_, App>) -> String {
    let mut out = String::new();
    for ClippedShape {
        clip_rect, shape, ..
    } in &harness.output().shapes
    {
        let _ = writeln!(out, "clip {clip_rect:?}");
        shape_lines(&mut out, 1, shape);
    }
    out
}

fn check(scene: &str, harness: &Harness<'_, App>) {
    let path = format!("{}/tests/painting/{scene}.txt", env!("CARGO_MANIFEST_DIR"));
    let actual = digest(harness);
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
        std::fs::create_dir_all(std::path::Path::new(&path).parent().unwrap()).unwrap();
        std::fs::write(&path, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{path}: {e}; run with UPDATE_SNAPSHOTS=1 to write it"));
    // Line by line: a checkout may have turned the files' line ends into CRLF.
    if !actual.lines().eq(expected.lines()) {
        let new = format!("{path}.new");
        std::fs::write(&new, &actual).unwrap();
        let first = actual
            .lines()
            .zip(expected.lines())
            .position(|(a, e)| a != e)
            .unwrap_or_else(|| actual.lines().count().min(expected.lines().count()));
        panic!(
            "{scene}: the shapes differ from line {} (see {new})",
            first + 1
        );
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn tabs_at_rest_and_hovered() {
    let mut harness = scene(false, OverlayType::Widgets);
    check("rest", &harness);
    let second = tab_rect(&harness, "Second");
    harness.hover_at(pos2(second.left() + 4.0, second.center().y));
    harness.run();
    check("tab_hovered", &harness);
    // The close button sits at the tab's right end.
    harness.hover_at(pos2(second.right() - 8.0, second.center().y));
    harness.run();
    check("close_hovered", &harness);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn separator_hovered() {
    let mut harness = scene(false, OverlayType::Widgets);
    let left = harness.state().dock_state.main_surface()[NodeIndex::root().left()]
        .rect()
        .unwrap();
    harness.hover_at(pos2(left.right() + 0.5, left.center().y));
    harness.run();
    check("separator_hovered", &harness);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn drop_overlays() {
    for (name, refuse, overlay) in [
        ("drop_widgets", false, OverlayType::Widgets),
        ("drop_areas", false, OverlayType::HighlightedAreas),
        ("drop_refused_widgets", true, OverlayType::Widgets),
        ("drop_refused_areas", true, OverlayType::HighlightedAreas),
    ] {
        let mut harness = scene(refuse, overlay);
        drag_first_to(&mut harness, third_body);
        check(name, &harness);
    }
    let mut harness = scene(false, OverlayType::HighlightedAreas);
    drag_first_to(&mut harness, |h| {
        let third = tab_rect(h, "Third");
        pos2(third.right() + 40.0, third.center().y)
    });
    check("drop_tab_bar", &harness);
    let mut harness = scene(false, OverlayType::HighlightedAreas);
    drag_first_to(&mut harness, |h| {
        let leaf = h.state().dock_state.main_surface()[NodeIndex::root().right()]
            .rect()
            .unwrap();
        pos2(leaf.right() - 20.0, leaf.center().y)
    });
    check("drop_split", &harness);
}
