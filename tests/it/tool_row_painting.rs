//! A toolbar row's painting: the default, shape by shape, and a painter of its own.
//!
//! The default's shapes are compared with `tests/painting/tool_row_<scene>.txt`. Run with
//! `UPDATE_SNAPSHOTS=1` to write the files again after an intended change.

use std::fmt::Write as _;
use std::sync::{Arc, Mutex};

use egui::accesskit::Role;
use egui::epaint::ClippedShape;
use egui::{Color32, Painter, Rect, Shape, TextureId, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use egui_stack::toolbar::{Tool, ToolDisplay, ToolKind, ToolRow};
use egui_stack::{DockPainter, PartState, ToolItemPaint, ToolMorePaint, ToolSeparatorPaint};

const NAME: &str = "painting";

fn icon(painter: &Painter, rect: Rect, color: Color32) {
    painter.rect_filled(rect.shrink(3.0), 0.0, color);
}

fn row(ui: &mut egui::Ui, painter: Option<&(dyn DockPainter + Sync)>) {
    let mut row = ToolRow::new(NAME)
        .title("Formatting")
        .display(ToolDisplay::Beside);
    if let Some(painter) = painter {
        row = row.painter(painter);
    }
    row.show(
        ui,
        [
            Tool::button("cut", "Cut").icon_painter(icon),
            // An image icon: the font atlas stands in for a texture.
            Tool::button("copy", "Copy")
                .icon(egui::Image::from_texture((
                    TextureId::default(),
                    vec2(16.0, 16.0),
                )))
                .enabled(false),
            Tool::separator(),
            Tool::toggle("bold", "Bold", true).icon_painter(icon),
            Tool::menu("view", "View", |ui| {
                ui.label("menu");
            }),
            Tool::button("save", "Save")
                .icon_painter(icon)
                .shortcut("Ctrl+S"),
            Tool::button("print", "Print").icon_painter(icon),
            Tool::toggle("wrap", "Wrap", true),
        ],
    );
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

fn check(scene: &str, harness: &Harness<'_>) {
    let mut actual = String::new();
    for ClippedShape {
        clip_rect, shape, ..
    } in &harness.output().shapes
    {
        let _ = writeln!(actual, "clip {clip_rect:?}");
        shape_lines(&mut actual, 1, shape);
    }
    let path = format!(
        "{}/tests/painting/tool_row_{scene}.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("UPDATE_SNAPSHOTS").is_some() {
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
fn the_default_painting_at_rest_hovered_and_focused() {
    // Narrow enough that the last tools go into the More button.
    let mut h = Harness::builder()
        .with_size([300.0, 120.0])
        .build_ui(|ui| row(ui, None));
    h.run();
    check("rest", &h);
    let cut = egui_stack::toolbar::laid_out(&h.ctx, NAME).unwrap().tools[0]
        .rect
        .unwrap();
    h.hover_at(cut.center());
    h.run();
    check("hovered", &h);
    h.event(egui::Event::PointerGone);
    h.get_by_role_and_label(Role::Button, "More tools: Formatting")
        .focus();
    h.run();
    check("more_focused", &h);
    // Its menu: the rows line up, a key binding at the end, a tick for a toggle on.
    h.key_press(egui::Key::Enter);
    h.run();
    check("more_open", &h);
}

/// What a tool is painted with: its kind, whether it shows an icon and a label, its mark,
/// whether in the menu, and its state.
type Item = (ToolKind, bool, bool, bool, bool, PartState);

/// Records what it is asked to paint, and paints nothing.
#[derive(Default)]
struct Recorder {
    items: Mutex<Vec<Item>>,
    more: Mutex<Vec<PartState>>,
    separators: Mutex<usize>,
}

impl DockPainter for Recorder {
    fn tool_item(&self, _painter: &Painter, item: &ToolItemPaint<'_>) {
        let seen = (
            item.kind,
            item.icon.is_some(),
            item.label.is_some(),
            item.mark.is_some(),
            item.in_menu,
            item.state,
        );
        self.items.lock().unwrap().push(seen);
    }

    fn tool_more(&self, _painter: &Painter, more: &ToolMorePaint<'_>) {
        self.more.lock().unwrap().push(more.state);
    }

    fn tool_separator(&self, _painter: &Painter, _separator: &ToolSeparatorPaint<'_>) {
        *self.separators.lock().unwrap() += 1;
    }
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_painter_of_its_own_paints_every_tool_the_more_button_and_its_menu() {
    let recorder = Arc::new(Recorder::default());
    let painter = Arc::clone(&recorder);
    let mut h = Harness::builder()
        .with_size([300.0, 200.0])
        .build_ui(move |ui| row(ui, Some(&*painter)));
    h.run();
    let items = recorder.items.lock().unwrap().clone();
    let last: Vec<_> = items[items.len() - 4..].to_vec();
    use PartState as S;
    assert_eq!(
        last,
        [
            (ToolKind::Button, true, true, false, false, S::NONE),
            (ToolKind::Button, true, true, false, false, S::DISABLED),
            (ToolKind::Toggle, true, true, false, false, S::SELECTED),
            (ToolKind::Menu, false, true, true, false, S::NONE),
        ]
    );
    assert_eq!(recorder.more.lock().unwrap().last(), Some(&S::NONE));
    assert!(*recorder.separators.lock().unwrap() > 0);
    // The More menu's rows go through the same painter, marked as in a menu.
    recorder.items.lock().unwrap().clear();
    h.get_by_role_and_label(Role::Button, "More tools: Formatting")
        .click();
    h.run();
    let items = recorder.items.lock().unwrap().clone();
    assert!(
        items.iter().any(
            |&(kind, icon, label, _, in_menu, _)| kind == ToolKind::Button
                && icon
                && label
                && in_menu
        ),
        "{items:?}"
    );
    let more = *recorder.more.lock().unwrap().last().unwrap();
    assert!(more.contains(S::SELECTED), "open: {more:?}");
}
