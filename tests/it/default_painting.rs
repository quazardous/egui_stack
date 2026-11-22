//! The dock's default painting, shape by shape: tabs, bars, separators and the drop overlay.
//!
//! Each scene's shapes are compared with `tests/painting/<scene>.txt`. Run with
//! `UPDATE_SNAPSHOTS=1` to write the files again after an intended change.

use std::fmt::Write as _;

use egui::epaint::ClippedShape;

use egui::{Id, Pos2, Rect, Shape, Ui, WidgetText, pos2, vec2};
use egui_kittest::{Harness, kittest::Queryable as _};
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
    classic: bool,
}

fn scene(refuse: bool, overlay: OverlayType) -> Harness<'static, App> {
    scene_with(refuse, overlay, false)
}

/// A scene in the former look of `Style::from_egui`: a border around each body and a
/// separator line at rest, set explicitly.
fn classic_scene() -> Harness<'static, App> {
    scene_with(false, OverlayType::Widgets, true)
}

fn scene_with(refuse: bool, overlay: OverlayType, classic: bool) -> Harness<'static, App> {
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
        classic,
    };
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        |ui, app: &mut App| {
            app.viewer.tab_rects.clear();
            let mut style = Style::from_egui(ui.style());
            style.overlay.overlay_type = app.overlay;
            if app.classic {
                let line = ui.visuals().widgets.noninteractive.bg_stroke;
                style.tab.tab_body.stroke = line;
                style.separator.color_idle = line.color;
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
    let leaf = harness.state().dock_state.main_surface()[NodeIndex(2)]
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
fn tab_focused() {
    let mut harness = scene(false, OverlayType::Widgets);
    let third = tab_rect(&harness, "Third");
    harness.hover_at(third.center());
    harness.step();
    harness.drag_at(third.center());
    harness.step();
    harness.drop_at(third.center());
    harness.run();
    assert!(harness.state().dock_state.focused_leaf().is_some());
    check("tab_focused", &harness);
}

/// A folded leaf shows only its tab bar, with no body below it: the bar and its tabs are
/// rounded at the bottom too.
#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn leaf_folded() {
    let mut harness = scene(false, OverlayType::Widgets);
    let collapse = harness
        .get_all_by_label("Collapse")
        .map(|button| button.rect())
        .max_by(|a, b| a.left().total_cmp(&b.left()))
        .unwrap();
    harness.hover_at(collapse.center());
    harness.step();
    harness.drag_at(collapse.center());
    harness.step();
    harness.drop_at(collapse.center());
    // Away from the button, so that it is painted at rest.
    harness.hover_at(pos2(1.0, 1.0));
    harness.run();
    assert!(
        harness.state().dock_state.main_surface()[NodeIndex(2)].is_collapsed(),
        "the leaf holding Third is folded"
    );
    check("leaf_folded", &harness);
    // The collapse button's highlight, at the bar's left end, keeps to its rounded corners.
    harness.hover_at(collapse.center());
    harness.run();
    check("collapse_hovered_folded", &harness);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn separator_hovered() {
    let mut harness = scene(false, OverlayType::Widgets);
    let left = harness.state().dock_state.main_surface()[NodeIndex(1)]
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
        ("drop_zones", false, OverlayType::Zones),
        ("drop_refused_zones", true, OverlayType::Zones),
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
        let leaf = h.state().dock_state.main_surface()[NodeIndex(2)]
            .rect()
            .unwrap();
        pos2(leaf.right() - 20.0, leaf.center().y)
    });
    check("drop_split", &harness);
    let mut harness = scene(false, OverlayType::Zones);
    drag_first_to(&mut harness, |h| {
        let leaf = h.state().dock_state.main_surface()[NodeIndex(2)]
            .rect()
            .unwrap();
        pos2(leaf.right() - 20.0, leaf.center().y)
    });
    check("drop_zone_split", &harness);
    let mut harness = scene(false, OverlayType::Zones);
    drag_first_to(&mut harness, |h| {
        let third = tab_rect(h, "Third");
        pos2(third.right() + 40.0, third.center().y)
    });
    check("drop_zone_tab_bar", &harness);
}

/// With the body stroke and the idle separator colour set explicitly, the default painter
/// paints what it painted before they became invisible by default: the shapes are those of the
/// former `rest` and `tab_focused` scenes.
#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn classic_look_by_style() {
    let harness = classic_scene();
    check("classic_rest", &harness);
    let mut harness = classic_scene();
    let third = tab_rect(&harness, "Third");
    harness.hover_at(third.center());
    harness.step();
    harness.drag_at(third.center());
    harness.step();
    harness.drop_at(third.center());
    harness.run();
    check("classic_tab_focused", &harness);
}
