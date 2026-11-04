//! A section stack's painting: the default, shape by shape, and a painter of its own.
//!
//! The default's shapes are compared with `tests/painting/sections_<scene>.txt`. Run with
//! `UPDATE_SNAPSHOTS=1` to write the files again after an intended change.

use std::fmt::Write as _;
use std::sync::Mutex;

use egui::accesskit::Role;
use egui::epaint::ClippedShape;
use egui::{Painter, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use egui_stack::sections::{Section, Sections, laid_out};
use egui_stack::{DockPainter, PartState, SectionBandPaint, SectionBoundaryPaint};

const NAME: &str = "painting";

fn stack(ui: &mut egui::Ui, painter: Option<&(dyn DockPainter + Sync)>) {
    let mut sections = Sections::new(NAME);
    if let Some(painter) = painter {
        sections = sections.painter(painter);
    }
    sections.show(ui, |stack| {
        stack.section(Section::new("live", "Live").count(3), |ui| {
            ui.label("build");
        });
        stack.section(Section::new("many", "Many").count(250), |ui| {
            ui.label("a row");
        });
        stack.section(Section::new("shut", "Shut").open(false), |ui| {
            ui.label("hidden");
        });
    });
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
        "{}/tests/painting/sections_{scene}.txt",
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
    let mut h = Harness::builder()
        .with_size([300.0, 300.0])
        .build_ui(|ui| stack(ui, None));
    h.run();
    check("rest", &h);
    let band = laid_out(&h.ctx, NAME)[1].band;
    h.hover_at(band.center());
    h.run();
    check("band_hovered", &h);
    h.event(egui::Event::PointerGone);
    h.get_by_role_and_label(Role::Splitter, "Resize Live")
        .focus();
    h.run();
    check("boundary_focused", &h);
}

/// A band as the painter is given it: its title, count, fold and state.
type Band = (String, Option<u64>, bool, PartState);

/// Records what it is asked to paint, and paints nothing.
#[derive(Default)]
struct Recorder {
    bands: Mutex<Vec<Band>>,
    boundaries: Mutex<Vec<PartState>>,
}

impl DockPainter for Recorder {
    fn section_band(&self, _painter: &Painter, band: &SectionBandPaint<'_>) {
        let seen = (band.title.to_owned(), band.count, band.open, band.state);
        self.bands.lock().unwrap().push(seen);
    }

    fn section_boundary(&self, _painter: &Painter, boundary: &SectionBoundaryPaint<'_>) {
        self.boundaries.lock().unwrap().push(boundary.state);
    }
}

#[test]
fn a_painter_of_its_own_paints_every_band_and_boundary() {
    let recorder: &'static Recorder = Box::leak(Box::default());
    let mut h = Harness::builder()
        .with_size([300.0, 300.0])
        .build_ui(move |ui| stack(ui, Some(recorder)));
    h.run();
    recorder.bands.lock().unwrap().clear();
    recorder.boundaries.lock().unwrap().clear();
    h.get_by_role_and_label(Role::Splitter, "Resize Live")
        .focus();
    h.step();
    let bands = recorder.bands.lock().unwrap().clone();
    let titles: Vec<_> = bands
        .iter()
        .map(|(title, count, open, _)| (title.as_str(), *count, *open))
        .collect();
    assert_eq!(
        titles[titles.len() - 3..],
        [
            ("LIVE", Some(3), true),
            ("MANY", Some(250), true),
            ("SHUT", None, false)
        ]
    );
    // One boundary between the two open sections, focused.
    let boundaries = recorder.boundaries.lock().unwrap().clone();
    assert_eq!(boundaries.last(), Some(&PartState::FOCUSED));
}
