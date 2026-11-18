//! A single dragged tab's ghost looks exactly like a whole group's: the same shadow,
//! translucency, tab bar and body hint, sized and clamped the same, holding just that tab. It
//! goes through `DockPainter::dragged_tab`, whose default paints it as `dragged_group` paints a
//! one-tab group; the tab itself leaves its bar while it is in the air.

use egui::{
    Color32, Event, Id, Modifiers, Pos2, Rect, Shape, Ui, UiBuilder, Vec2, WidgetText,
    epaint::ClippedShape, pos2, vec2,
};
use egui_kittest::{Harness, kittest::Queryable as _};
use std::sync::{LazyLock, Mutex};

use egui_stack::{
    DockArea, DockPainter, DockState, DraggedGroupPaint, NodeIndex, StylePainter, TabViewer,
};

/// The colour of the preview a [`Viewer`] paints in the ghost's body.
const PREVIEW: Color32 = Color32::from_rgb(0x12, 0x34, 0x56);

/// The ghost's size a previewing [`Viewer`] asks for.
const PREVIEW_SIZE: Vec2 = vec2(250.0, 150.0);

#[derive(Default)]
struct Viewer {
    /// Paints its own ghost body, at its own ghost size.
    preview: bool,
    /// The tab and rect of each `ghost_body` call.
    bodies: Vec<(String, Rect)>,
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

    fn ghost_size(&mut self, _tab: &String, default: Vec2) -> Vec2 {
        if self.preview { PREVIEW_SIZE } else { default }
    }

    fn ghost_body(&mut self, ui: &mut Ui, tab: &String, rect: Rect) -> bool {
        self.bodies.push((tab.clone(), rect));
        if self.preview {
            ui.painter().rect_filled(rect.shrink(4.0), 0.0, PREVIEW);
        }
        self.preview
    }
}

struct App {
    dock_state: DockState<String>,
    viewer: Viewer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Tab,
    Group,
}

/// A ghost painted: by which method, how many tabs it held, and the shapes the default
/// painting added to its layer.
#[derive(Clone, Debug)]
struct Ghost {
    kind: Kind,
    count: usize,
    rect: Rect,
    body: Rect,
    body_painted: bool,
    shapes: Vec<ClippedShape>,
}

/// A painter that records the ghosts it paints, painted as by default.
#[derive(Default)]
struct Recorder(Mutex<Vec<Ghost>>);

impl Recorder {
    fn paint(
        &self,
        kind: Kind,
        painter: &egui::Painter,
        ghost: &DraggedGroupPaint<'_>,
        default: impl FnOnce(),
    ) {
        let layer = painter.layer_id();
        let entries = |ctx: &egui::Context| -> Vec<ClippedShape> {
            ctx.graphics(|g| {
                g.get(layer)
                    .map(|list| list.all_entries().cloned().collect())
                    .unwrap_or_default()
            })
        };
        let before = entries(painter.ctx()).len();
        default();
        let shapes = entries(painter.ctx()).split_off(before);
        self.0.lock().unwrap().push(Ghost {
            kind,
            count: ghost.count,
            rect: ghost.rect,
            body: ghost.body,
            body_painted: ghost.body_painted,
            shapes,
        });
    }
}

impl DockPainter for Recorder {
    fn dragged_tab(&self, painter: &egui::Painter, tab: &DraggedGroupPaint<'_>) {
        self.paint(Kind::Tab, painter, tab, || {
            StylePainter.dragged_tab(painter, tab);
        });
    }

    fn dragged_group(&self, painter: &egui::Painter, group: &DraggedGroupPaint<'_>) {
        self.paint(Kind::Group, painter, group, || {
            StylePainter.dragged_group(painter, group);
        });
    }
}

const TARGET: Pos2 = pos2(150.0, 250.0);

/// `Alpha` on the left, `right` on the right; the dock takes the left 600 points of an
/// 800 × 400 screen. Drags `Solo` (with `modifiers` held) to `TARGET` and returns the last ghost
/// painted while it is held there, and the harness, still dragging. With `preview`, the viewer
/// paints the ghost's body.
fn drag_solo(
    right: &[&str],
    modifiers: Modifiers,
    preview: bool,
    recorder: &'static Recorder,
) -> (Ghost, Harness<'static, App>) {
    let mut dock_state = DockState::new(vec!["Alpha".to_owned()]);
    dock_state.main_surface_mut().split_right(
        NodeIndex::root(),
        0.5,
        right.iter().map(|s| (*s).to_owned()).collect(),
    );
    let mut harness = Harness::builder().with_size([800.0, 400.0]).build_ui_state(
        move |ui, app: &mut App| {
            let dock = Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0));
            ui.scope_builder(UiBuilder::new().max_rect(dock), |ui| {
                DockArea::new(&mut app.dock_state)
                    .painter(recorder)
                    .show_inside(ui, &mut app.viewer);
            });
        },
        App {
            dock_state,
            viewer: Viewer {
                preview,
                bodies: Vec::new(),
            },
        },
    );
    harness.run();
    let start = harness.get_by_label("Solo").rect().center();
    harness.event(Event::ModifiersChanged(modifiers));
    harness.hover_at(start);
    harness.step();
    harness.drag_at(start);
    harness.step();
    for step in 1..=10 {
        harness.hover_at(start.lerp(TARGET, step as f32 / 10.0));
        harness.step();
    }
    for _ in 0..3 {
        harness.hover_at(TARGET);
        harness.step();
    }
    let ghost = recorder
        .0
        .lock()
        .unwrap()
        .last()
        .cloned()
        .expect("a ghost painted");
    (ghost, harness)
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_tab_ghost_paints_the_same_shapes_as_a_one_tab_group_ghost() {
    static TAB: LazyLock<Recorder> = LazyLock::new(Recorder::default);
    static GROUP: LazyLock<Recorder> = LazyLock::new(Recorder::default);
    let (tab, _) = drag_solo(&["Solo", "Other"], Modifiers::NONE, false, &TAB);
    let (group, _) = drag_solo(&["Solo"], Modifiers::SHIFT, false, &GROUP);

    assert_eq!(
        tab.kind,
        Kind::Tab,
        "a tab's ghost goes through dragged_tab"
    );
    assert_eq!(group.kind, Kind::Group);
    assert_eq!(tab.count, 1, "the tab's ghost holds just that tab");
    assert_eq!(group.count, 1);
    assert!(
        tab.rect.expand(1.0).contains(TARGET),
        "under the pointer: {:?}",
        tab.rect
    );
    assert_eq!(tab.rect, group.rect, "sized and placed alike");
    assert!(!tab.shapes.is_empty(), "the default paints the ghost");
    assert_eq!(
        tab.shapes, group.shapes,
        "the tab's ghost paints a one-tab group's shapes"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_dragged_tab_leaves_its_bar() {
    static TAB: LazyLock<Recorder> = LazyLock::new(Recorder::default);
    let (_, harness) = drag_solo(&["Solo", "Other"], Modifiers::NONE, false, &TAB);
    // The title is painted once, in the ghost under the pointer, not at the top of the dock.
    let surface = harness.state().dock_state.main_surface();
    let (node, _) = surface.find_tab(&"Other".to_owned()).unwrap();
    let leaf = surface[node].rect().unwrap();
    let mut solo = Vec::new();
    visit_texts(&harness.output().shapes, "Solo", &mut solo);
    assert!(!solo.is_empty(), "the ghost shows the title");
    assert!(
        solo.iter().all(|rect| rect.top() > leaf.top() + 40.0),
        "no title left in the bar ({leaf:?}): {solo:?}"
    );
}

/// Where the shapes drawing `text` are, in `shapes`.
fn visit_texts(shapes: &[ClippedShape], text: &str, rects: &mut Vec<Rect>) {
    fn visit(shape: &egui::Shape, clip: Rect, text: &str, rects: &mut Vec<Rect>) {
        match shape {
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    visit(shape, clip, text, rects);
                }
            }
            egui::Shape::Text(text_shape) if text_shape.galley.text() == text => {
                let rect = shape.visual_bounding_rect().intersect(clip);
                if rect.is_positive() {
                    rects.push(rect);
                }
            }
            _ => {}
        }
    }
    for clipped in shapes {
        visit(&clipped.shape, clipped.clip_rect, text, rects);
    }
}

/// Whether `shapes` hold a rect filled with `color`, and where.
fn filled(shapes: &[ClippedShape], color: Color32) -> Vec<Rect> {
    fn visit(shape: &Shape, color: Color32, rects: &mut Vec<Rect>) {
        match shape {
            Shape::Vec(shapes) => {
                for shape in shapes {
                    visit(shape, color, rects);
                }
            }
            Shape::Rect(rect) if rect.fill == color => rects.push(rect.rect),
            _ => {}
        }
    }
    let mut rects = Vec::new();
    for clipped in shapes {
        visit(&clipped.shape, color, &mut rects);
    }
    rects
}

/// The application's ghost body: called for `shown` with the ghost's body rect, painted over
/// the ghost (translucent, as the rest of it), at the size it asked for; the painter's plain
/// body hint left out.
fn check_preview(ghost: &Ghost, harness: &Harness<'_, App>, shown: &str) {
    assert_eq!(ghost.rect.size(), PREVIEW_SIZE, "the viewer's ghost size");
    assert!(
        ghost.body_painted,
        "the painter is told the body is painted"
    );
    let (tab, rect) = harness
        .state()
        .viewer
        .bodies
        .last()
        .cloned()
        .expect("ghost_body called");
    assert_eq!(tab, shown, "the body of the shown tab");
    assert_eq!(rect, ghost.body, "ghost_body gets the ghost's body");
    let bg = egui_stack::Style::from_egui(&egui::Style::default())
        .tab
        .tab_body
        .bg_fill;
    let hint = bg.gamma_multiply(0.7);
    assert!(
        !filled(&ghost.shapes, bg)
            .iter()
            .chain(filled(&ghost.shapes, hint).iter())
            .any(|r| *r == ghost.body),
        "no built-in body hint: {:?}",
        ghost.shapes
    );
    let preview = filled(&harness.output().shapes, PREVIEW.gamma_multiply(0.7));
    assert!(
        preview.iter().any(|r| *r == ghost.body.shrink(4.0)),
        "the preview is painted, translucent, in the body: {preview:?}"
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_viewer_paints_the_tab_ghost_body() {
    static TAB: LazyLock<Recorder> = LazyLock::new(Recorder::default);
    let (ghost, harness) = drag_solo(&["Solo", "Other"], Modifiers::NONE, true, &TAB);
    assert_eq!(ghost.kind, Kind::Tab);
    check_preview(&ghost, &harness, "Solo");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_viewer_paints_the_group_ghost_body_of_the_active_tab() {
    static GROUP: LazyLock<Recorder> = LazyLock::new(Recorder::default);
    // `Solo` dragged with Shift carries its group; `Solo`, the first tab, is the active one.
    let (ghost, harness) = drag_solo(&["Solo", "Other"], Modifiers::SHIFT, true, &GROUP);
    assert_eq!(ghost.kind, Kind::Group);
    assert_eq!(ghost.count, 2);
    check_preview(&ghost, &harness, "Solo");
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn by_default_the_painter_paints_the_body_hint() {
    static TAB: LazyLock<Recorder> = LazyLock::new(Recorder::default);
    let (ghost, harness) = drag_solo(&["Solo", "Other"], Modifiers::NONE, false, &TAB);
    assert!(!ghost.body_painted);
    assert_eq!(
        harness.state().viewer.bodies.last().map(|(_, r)| *r),
        Some(ghost.body),
        "ghost_body is offered the body"
    );
    assert!(
        filled(&harness.output().shapes, PREVIEW.gamma_multiply(0.7)).is_empty(),
        "nothing of the viewer's"
    );
}
