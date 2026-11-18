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
        let commands = out
            .viewport_output
            .get(&ids.this)
            .map(|o| o.commands.clone())
            .unwrap_or_default();
        h.borrow_mut().shown.push((ids.this, builder, commands));
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
    /// The commands each window's passes sent, since the test last took them.
    commands: BTreeMap<ViewportId, Vec<ViewportCommand>>,
    positions_hidden: bool,
    time: f64,
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
            commands: BTreeMap::new(),
            positions_hidden: false,
            time: 0.0,
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
        // Events are told once.
        for info in self.info.values_mut() {
            info.events.clear();
        }
        let mut immediate = self.immediate.borrow_mut();
        immediate.inputs.clear();
        let shown = std::mem::take(&mut immediate.shown);
        drop(immediate);
        self.shown.clear();
        for (viewport, builder, commands) in shown {
            self.shown.insert(viewport);
            self.info
                .entry(viewport)
                .or_insert_with(|| Self::opened_info(&builder));
            self.builders.entry(viewport).or_insert(builder);
            self.commands.entry(viewport).or_default().extend(commands);
        }
        for (viewport, output) in out.viewport_output {
            if viewport != ViewportId::ROOT && !output.commands.is_empty() {
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
    pub fn drag(&mut self, from: Pos2, to: Pos2, mut app: impl FnMut(&mut Ui)) {
        let root = ViewportId::ROOT;
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.queue(root, Event::PointerMoved(from));
        self.pass(&mut app);
        self.queue(root, button(from, true));
        self.pass(&mut app);
        for step in 1..=10 {
            let at = from + (to - from) * (step as f32 / 10.0);
            self.queue(root, Event::PointerMoved(at));
            self.pass(&mut app);
        }
        // Held there a while: a leaf hovered on the way is kept a moment.
        for _ in 0..30 {
            self.queue(root, Event::PointerMoved(to));
            self.pass(&mut app);
        }
        self.queue(root, button(to, false));
        self.pass(&mut app);
        self.passes(3, &mut app);
    }
}

impl Drop for Multi {
    fn drop(&mut self) {
        IMMEDIATE.with_borrow_mut(|h| *h = None);
    }
}
