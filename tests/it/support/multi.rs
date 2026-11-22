//! A headless harness with real viewports, as eframe runs them, without a GPU.
//!
//! kittest has one window and embeds the others in it. [`Multi`] runs an egui context whose
//! immediate viewports are real: the harness is the context's immediate renderer, so a window
//! shown with `Context::show_viewport_immediate` runs a pass of its own, with its own input
//! (its size, its position, a close request), inside the main window's pass, as eframe runs
//! it. A window is listed from the pass after it is first shown, and leaves the list at the
//! first main pass that no longer shows it.

#![allow(dead_code)] // Each test file uses part of it.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use egui::epaint::ClippedShape;

use egui::{
    Context, Event, ImmediateViewport, Modifiers, PointerButton, Pos2, RawInput, Rect, Ui, Vec2,
    ViewportBuilder, ViewportCommand, ViewportEvent, ViewportId, ViewportInfo,
};

/// Each window's size at first.
pub const SIZE: Vec2 = Vec2::new(640.0, 480.0);

/// The immediate windows' side: the input each takes in its next pass, and what the passes
/// run inside the last main pass gave.
#[derive(Default)]
struct Immediate {
    inputs: BTreeMap<ViewportId, RawInput>,
    shown: Vec<(ViewportId, ViewportBuilder, Vec<ViewportCommand>)>,
    shapes: BTreeMap<ViewportId, Vec<ClippedShape>>,
    /// The labelled widgets of each window's last pass, from AccessKit, and whether each
    /// takes the focus.
    labels: BTreeMap<ViewportId, Vec<(String, Rect, bool)>>,
    /// The cursor each window asked for in its last pass.
    cursors: BTreeMap<ViewportId, egui::CursorIcon>,
}

/// The labelled widgets of a pass's AccessKit update, with their bounds (scale 1) and whether
/// each takes the focus.
fn labels_of(output: &egui::PlatformOutput) -> Vec<(String, Rect, bool)> {
    let Some(update) = &output.accesskit_update else {
        return Vec::new();
    };
    update
        .nodes
        .iter()
        .filter_map(|(_, node)| {
            let label = node.label()?;
            let b = node.bounds()?;
            Some((
                label.to_owned(),
                Rect::from_min_max(
                    Pos2::new(b.x0 as f32, b.y0 as f32),
                    Pos2::new(b.x1 as f32, b.y1 as f32),
                ),
                node.supports_action(egui::accesskit::Action::Focus),
            ))
        })
        .collect()
}

thread_local! {
    /// The harness of this thread: egui's immediate renderer is one per thread.
    static IMMEDIATE: RefCell<Option<Rc<RefCell<Immediate>>>> = const { RefCell::new(None) };
}

/// The immediate renderer: runs a window's pass with the input the harness prepared.
fn render_immediate(ctx: &Context, viewport: ImmediateViewport<'_>) {
    let ImmediateViewport {
        ids,
        builder,
        mut viewport_ui_cb,
    } = viewport;
    let harness = IMMEDIATE.with_borrow(Clone::clone);
    let input = harness
        .as_ref()
        .and_then(|h| h.borrow_mut().inputs.remove(&ids.this))
        .unwrap_or_else(|| {
            // A window shown for the first time: the system makes it as its builder says.
            let info = Multi::opened_info(&builder);
            let size = info.inner_rect.map_or(SIZE, |r| r.size());
            RawInput {
                viewport_id: ids.this,
                viewports: std::iter::once((ids.this, info)).collect(),
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                ..Default::default()
            }
        });
    let mut out = ctx.run_ui(input, &mut viewport_ui_cb);
    out.textures_delta.clear();
    if let Some(h) = harness {
        {
            let mut h = h.borrow_mut();
            h.labels.insert(ids.this, labels_of(&out.platform_output));
            h.cursors.insert(ids.this, out.platform_output.cursor_icon);
        }
        let commands = out
            .viewport_output
            .get(&ids.this)
            .map(|o| o.commands.clone())
            .unwrap_or_default();
        let mut h = h.borrow_mut();
        h.shapes.insert(ids.this, out.shapes);
        h.shown.push((ids.this, builder, commands));
    }
}

/// A context driven pass by pass, each window with its own input.
pub struct Multi {
    ctx: Context,
    immediate: Rc<RefCell<Immediate>>,
    info: BTreeMap<ViewportId, ViewportInfo>,
    queued: BTreeMap<ViewportId, Vec<Event>>,
    /// The windows the last main pass showed.
    shown: BTreeSet<ViewportId>,
    /// The builder each window was first shown with.
    builders: BTreeMap<ViewportId, ViewportBuilder>,
    /// The builder each window was last shown with.
    last_builders: BTreeMap<ViewportId, ViewportBuilder>,
    /// The modifiers held, given with the button's events.
    modifiers: Modifiers,
    /// The commands each window's passes sent, since the test last took them.
    commands: BTreeMap<ViewportId, Vec<ViewportCommand>>,
    positions_hidden: bool,
    time: f64,
    /// What each window painted in its last pass.
    shapes: BTreeMap<ViewportId, Vec<ClippedShape>>,
}

impl Default for Multi {
    fn default() -> Self {
        Self::new()
    }
}

impl Multi {
    pub fn new() -> Self {
        let ctx = Context::default();
        ctx.set_embed_viewports(false);
        ctx.enable_accesskit();
        let immediate = Rc::new(RefCell::new(Immediate::default()));
        IMMEDIATE.with_borrow_mut(|h| *h = Some(immediate.clone()));
        Context::set_immediate_viewport_renderer(render_immediate);
        let mut info = BTreeMap::new();
        let mut root = Self::fresh_info();
        root.outer_rect = Some(Rect::from_min_size(Pos2::ZERO, SIZE));
        root.inner_rect = root.outer_rect;
        info.insert(ViewportId::ROOT, root);
        Self {
            ctx,
            immediate,
            info,
            queued: BTreeMap::new(),
            shown: BTreeSet::new(),
            builders: BTreeMap::new(),
            last_builders: BTreeMap::new(),
            modifiers: Modifiers::NONE,
            commands: BTreeMap::new(),
            positions_hidden: false,
            time: 0.0,
            shapes: BTreeMap::new(),
        }
    }

    fn fresh_info() -> ViewportInfo {
        ViewportInfo {
            native_pixels_per_point: Some(1.0),
            inner_rect: Some(Rect::from_min_size(Pos2::ZERO, SIZE)),
            focused: Some(false),
            ..Default::default()
        }
    }

    /// What the system tells of a window it opened with `builder`: its size, and its
    /// position (at the desktop's corner when the builder gives none).
    fn opened_info(builder: &ViewportBuilder) -> ViewportInfo {
        let rect = Rect::from_min_size(
            builder.position.unwrap_or(Pos2::ZERO),
            builder.inner_size.unwrap_or(SIZE),
        );
        ViewportInfo {
            inner_rect: Some(rect),
            outer_rect: Some(rect),
            ..Self::fresh_info()
        }
    }

    /// What a window painted in the last main pass, empty if it was not shown.
    pub fn shapes(&self, viewport: ViewportId) -> &[ClippedShape] {
        self.shapes.get(&viewport).map_or(&[], Vec::as_slice)
    }

    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    /// The windows the last main pass showed (the main one not included).
    pub fn shown(&self) -> &BTreeSet<ViewportId> {
        &self.shown
    }

    /// The builder a window was first shown with.
    pub fn builder(&self, viewport: ViewportId) -> Option<&ViewportBuilder> {
        self.builders.get(&viewport)
    }

    /// The builder a window was last shown with.
    pub fn last_builder(&self, viewport: ViewportId) -> Option<&ViewportBuilder> {
        self.last_builders.get(&viewport)
    }

    /// Holds `modifiers` from now on, told to a window in its next pass.
    pub fn hold(&mut self, viewport: ViewportId, modifiers: Modifiers) {
        self.modifiers = modifiers;
        self.queue(viewport, Event::ModifiersChanged(modifiers));
    }

    /// Puts a window's top-left at `physical` on the desktop (physical pixels) on a screen of
    /// scale `scale`, as the system would after the user moved it there: its size in points is
    /// kept, as Windows and Wayland keep it across scales.
    pub fn place(&mut self, viewport: ViewportId, physical: Pos2, scale: f32) {
        let info = self.info.entry(viewport).or_insert_with(Self::fresh_info);
        let size = info.inner_rect.map_or(SIZE, |r| r.size());
        let rect = Rect::from_min_size((physical.to_vec2() / scale).to_pos2(), size);
        info.native_pixels_per_point = Some(scale);
        info.outer_rect = Some(rect);
        info.inner_rect = Some(rect);
    }

    /// Where the widget labelled `label` was in a window's last pass (its points, scale 1).
    pub fn find(&self, viewport: ViewportId, label: &str) -> Option<Rect> {
        self.immediate
            .borrow()
            .labels
            .get(&viewport)?
            .iter()
            .find(|(l, _, _)| l == label)
            .map(|(_, rect, _)| *rect)
    }

    /// Every labelled widget of a window's last pass, with where it was (its points, scale 1).
    pub fn labels(&self, viewport: ViewportId) -> Vec<(String, Rect)> {
        self.immediate
            .borrow()
            .labels
            .get(&viewport)
            .map(|labels| labels.iter().map(|(l, r, _)| (l.clone(), *r)).collect())
            .unwrap_or_default()
    }

    /// Whether the widget labelled `label` took the focus in a window's last pass (AccessKit's
    /// `Focus` action), as a keyboard stop.
    pub fn focusable(&self, viewport: ViewportId, label: &str) -> Option<bool> {
        self.immediate
            .borrow()
            .labels
            .get(&viewport)?
            .iter()
            .find(|(l, _, _)| l == label)
            .map(|(_, _, focusable)| *focusable)
    }

    /// Where a window's content is on the desktop (points at scale 1), as the system tells.
    pub fn inner_rect(&self, viewport: ViewportId) -> Option<Rect> {
        self.info.get(&viewport)?.inner_rect
    }

    /// The cursor a window asked for in its last pass.
    pub fn cursor(&self, viewport: ViewportId) -> Option<egui::CursorIcon> {
        self.immediate.borrow().cursors.get(&viewport).copied()
    }

    /// The commands a window's passes sent since the last call.
    pub fn take_commands(&mut self, viewport: ViewportId) -> Vec<ViewportCommand> {
        self.commands.remove(&viewport).unwrap_or_default()
    }

    /// The system tells no window's position from now on, as Wayland.
    pub fn hide_positions(&mut self) {
        self.positions_hidden = true;
    }

    /// Puts a window at `at` on the desktop (points at scale 1), as the system would after
    /// the user moved it.
    pub fn set_outer_position(&mut self, viewport: ViewportId, at: Pos2) {
        let info = self.info.entry(viewport).or_insert_with(Self::fresh_info);
        let size = info.inner_rect.map_or(SIZE, |r| r.size());
        let rect = Rect::from_min_size(at, size);
        info.outer_rect = Some(rect);
        info.inner_rect = Some(rect);
    }

    /// The user asks to close a window, seen in its next pass.
    pub fn request_close(&mut self, viewport: ViewportId) {
        self.info
            .entry(viewport)
            .or_insert_with(Self::fresh_info)
            .events
            .push(ViewportEvent::Close);
    }

    /// Queues an event for a window's next pass.
    pub fn queue(&mut self, viewport: ViewportId, event: Event) {
        self.queued.entry(viewport).or_default().push(event);
    }

    fn input(&mut self, viewport: ViewportId) -> RawInput {
        let info = self.info.entry(viewport).or_insert_with(Self::fresh_info);
        let size = info.inner_rect.map_or(SIZE, |r| r.size());
        let hidden = self.positions_hidden;
        let viewports = self
            .info
            .iter()
            .map(|(v, i)| {
                let mut i = i.clone();
                if hidden {
                    i.inner_rect = None;
                    i.outer_rect = None;
                }
                (*v, i)
            })
            .collect();
        RawInput {
            viewport_id: viewport,
            viewports,
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            time: Some(self.time),
            events: self.queued.remove(&viewport).unwrap_or_default(),
            ..Default::default()
        }
    }

    /// Runs the main window's pass (`app`), and inside it the passes of the windows it shows.
    pub fn pass(&mut self, mut app: impl FnMut(&mut Ui)) {
        self.time += 1.0 / 60.0;
        let mut windows: BTreeSet<ViewportId> = self.shown.clone();
        windows.extend(self.queued.keys().copied());
        windows.extend(
            self.info
                .iter()
                .filter(|(_, i)| !i.events.is_empty())
                .map(|(v, _)| *v),
        );
        windows.remove(&ViewportId::ROOT);
        for viewport in windows {
            let input = self.input(viewport);
            self.immediate.borrow_mut().inputs.insert(viewport, input);
        }
        let input = self.input(ViewportId::ROOT);
        let mut out = self.ctx.run_ui(input, &mut app);
        out.textures_delta.clear();
        {
            let mut immediate = self.immediate.borrow_mut();
            immediate
                .labels
                .insert(ViewportId::ROOT, labels_of(&out.platform_output));
            immediate
                .cursors
                .insert(ViewportId::ROOT, out.platform_output.cursor_icon);
        }
        // Events are told once.
        for info in self.info.values_mut() {
            info.events.clear();
        }
        let mut immediate = self.immediate.borrow_mut();
        immediate.inputs.clear();
        let shown = std::mem::take(&mut immediate.shown);
        self.shapes = std::mem::take(&mut immediate.shapes);
        self.shapes
            .insert(ViewportId::ROOT, std::mem::take(&mut out.shapes));
        drop(immediate);
        self.shown.clear();
        for (viewport, builder, commands) in shown {
            self.shown.insert(viewport);
            self.info
                .entry(viewport)
                .or_insert_with(|| Self::opened_info(&builder));
            self.builders.entry(viewport).or_insert(builder.clone());
            self.last_builders.insert(viewport, builder);
            // The system moves a window told to.
            for command in &commands {
                if let ViewportCommand::OuterPosition(at) = command {
                    self.set_outer_position(viewport, *at);
                }
            }
            self.commands.entry(viewport).or_default().extend(commands);
        }
        for (viewport, output) in out.viewport_output {
            if viewport != ViewportId::ROOT && !output.commands.is_empty() {
                for command in &output.commands {
                    if let ViewportCommand::OuterPosition(at) = command {
                        self.set_outer_position(viewport, *at);
                    }
                }
                self.commands
                    .entry(viewport)
                    .or_default()
                    .extend(output.commands);
            }
        }
        // A window no longer shown is gone.
        let gone: Vec<ViewportId> = self
            .info
            .keys()
            .copied()
            .filter(|v| *v != ViewportId::ROOT && !self.shown.contains(v))
            .collect();
        for viewport in gone {
            self.info.remove(&viewport);
        }
    }

    /// Runs `n` main passes.
    pub fn passes(&mut self, n: usize, mut app: impl FnMut(&mut Ui)) {
        for _ in 0..n {
            self.pass(&mut app);
        }
    }

    /// Drags in the main window from `from` to `to`, a step at a time, and releases there.
    pub fn drag(&mut self, from: Pos2, to: Pos2, app: impl FnMut(&mut Ui)) {
        self.drag_in(ViewportId::ROOT, from, to, app);
    }

    fn button(&self, pos: Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: self.modifiers,
        }
    }

    /// Presses the button at `at` in a window (its points), the pointer moved there first.
    pub fn press(&mut self, viewport: ViewportId, at: Pos2, mut app: impl FnMut(&mut Ui)) {
        self.queue(viewport, Event::PointerMoved(at));
        self.pass(&mut app);
        let press = self.button(at, true);
        self.queue(viewport, press);
        self.pass(&mut app);
    }

    /// Moves the held pointer from `from` to `to` in a window's points, a step at a time, as
    /// the system tells the window a drag began in (`to` may be past its edges), then holds it
    /// there a while (a leaf hovered on the way is kept a moment).
    pub fn move_held(
        &mut self,
        viewport: ViewportId,
        from: Pos2,
        to: Pos2,
        mut app: impl FnMut(&mut Ui),
    ) {
        for step in 1..=10 {
            let at = from + (to - from) * (step as f32 / 10.0);
            self.queue(viewport, Event::PointerMoved(at));
            self.pass(&mut app);
        }
        for _ in 0..30 {
            self.queue(viewport, Event::PointerMoved(to));
            self.pass(&mut app);
        }
    }

    /// Releases the button at `at` in a window's points.
    pub fn release(&mut self, viewport: ViewportId, at: Pos2, mut app: impl FnMut(&mut Ui)) {
        let release = self.button(at, false);
        self.queue(viewport, release);
        self.pass(&mut app);
    }

    /// Drags in a window from `from` to `to` (its points, `to` maybe past its edges), a step at
    /// a time, and releases there.
    pub fn drag_in(
        &mut self,
        viewport: ViewportId,
        from: Pos2,
        to: Pos2,
        mut app: impl FnMut(&mut Ui),
    ) {
        self.press(viewport, from, &mut app);
        self.move_held(viewport, from, to, &mut app);
        self.release(viewport, to, &mut app);
        self.passes(3, &mut app);
    }
}

impl Drop for Multi {
    fn drop(&mut self) {
        IMMEDIATE.with_borrow_mut(|h| *h = None);
    }
}
