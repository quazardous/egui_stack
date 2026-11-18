//! Floats drawn: the docks of a [`DockSpace`] the layout's [`Float`]s show, over the main
//! window ([`FloatHost::InWindow`]) or in native windows the application hosts
//! ([`FloatHost::Native`], through a [`ViewportHost`]). See [`FloatsArea`].

use std::collections::{BTreeMap, BTreeSet, HashMap};

use egui::{Context, Id, LayerId, Rect, Ui, ViewportBuilder, ViewportCommand, ViewportId, vec2};

use crate::layout::{Float, FloatHost, Layout, Place, Screen, ScreenGone};
use crate::{DockArea, DockSpace, Docked, Style, Surface, TabIndex, TabViewer, Tree};

/// What hosts the native windows of [`FloatHost::Native`] floats: the application, which
/// creates them, places them and renders them (egui_stack never creates a window or a
/// rendering surface).
///
/// Every method has a default, on egui's immediate viewports
/// ([`Context::show_viewport_immediate`]): [`EguiViewports`] is that host. An application that
/// pairs something with a window (a second rendering surface, display settings) keeps its own
/// viewport ids stable with [`viewport_id`](Self::viewport_id), and tells the screens it knows
/// with [`screens`](Self::screens).
pub trait ViewportHost {
    /// The viewport of the float keyed `key`: stable for as long as the float is open, so the
    /// application can pair anything with it.
    fn viewport_id(&self, key: &str) -> ViewportId {
        ViewportId::from_hash_of(("egui_stack.float", key))
    }

    /// The builder of the float's native window, before egui_stack sets what its place says
    /// (position, size, maximized, full screen) where the builder leaves it unset. By default
    /// a window titled with the float's key.
    fn builder(&mut self, float: &Float) -> ViewportBuilder {
        ViewportBuilder::default().with_title(float.key.clone())
    }

    /// Shows the viewport `id` with `builder`, `body` drawing its content. By default an
    /// immediate viewport ([`Context::show_viewport_immediate`]); a backend without several
    /// viewports draws it embedded, as an `egui::Window`.
    fn show(
        &mut self,
        ctx: &Context,
        id: ViewportId,
        builder: ViewportBuilder,
        body: &mut dyn FnMut(&mut Ui),
    ) {
        ctx.show_viewport_immediate(id, builder, |ui, _| body(ui));
    }

    /// The screens (monitors) the system has, named stably, in desktop physical pixels. A
    /// change of them brings each native float back onto them
    /// ([`Layout::repair_floats`]). By default none: nothing is repaired, and a position is
    /// never invented.
    fn screens(&self) -> Vec<Screen> {
        Vec::new()
    }

    /// The outer rect of the viewport `id` on the desktop, in physical pixels; `None` where
    /// the system does not tell window positions (Wayland). By default what egui's input says
    /// of it ([`egui::ViewportInfo::outer_rect`]).
    fn outer_rect(&self, ctx: &Context, id: ViewportId) -> Option<Rect> {
        let zoom = ctx.zoom_factor();
        ctx.input(|i| {
            let info = i.raw.viewports.get(&id)?;
            let rect = info.outer_rect?;
            let scale = info.native_pixels_per_point.unwrap_or(1.0) * zoom;
            Some(Rect::from_min_max(
                (rect.min.to_vec2() * scale).to_pos2(),
                (rect.max.to_vec2() * scale).to_pos2(),
            ))
        })
    }
}

/// The [`ViewportHost`] on egui's immediate viewports, with every default: no screens known.
#[derive(Clone, Copy, Debug, Default)]
pub struct EguiViewports;

impl ViewportHost for EguiViewports {}

/// What becomes of a float's tabs when the user closes it, from
/// [`TabViewer::close_policy`] (or, with the `bus` feature,
/// the `stack.window.close` request).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum ClosePolicy {
    /// Each tab goes back where it came from ([`TabOrigin`](crate::TabOrigin)): beside the tab
    /// it was next to, else in its dock's focused (or first) leaf; a tab with no origin, or
    /// whose dock is gone, goes to the first dock no float shows.
    #[default]
    ReturnToOrigin,
    /// The tabs go into the dock of that name, after its focused (or first) leaf's tabs; to
    /// their origin if the space has no such dock.
    Into(String),
    /// The tabs close with the window.
    CloseTabs,
    /// The window stays open: the close is cancelled.
    Keep,
}

/// A float the user closes, for [`TabViewer::close_policy`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct FloatClose {
    /// The float's key.
    pub key: String,
    /// The dock it shows.
    pub dock: String,
    /// Its tabs, by [`TabViewer::id`], in order.
    pub tabs: Vec<Id>,
}

/// A float's life, from [`DockSpace::take_float_events`]; layout changes are reported apart
/// ([`LayoutEvent`](crate::layout::LayoutEvent)).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum FloatEvent {
    /// A float opened from a tab or a group moved out of a dock.
    Detached {
        /// The float's key.
        key: String,
        /// The dock it shows, made for it.
        dock: String,
        /// Its tabs, by [`TabViewer::id`].
        tabs: Vec<Id>,
        /// What hosts it.
        host: FloatHost,
    },
    /// A float closed.
    Closed {
        /// The float's key.
        key: String,
        /// The dock it showed.
        dock: String,
        /// The policy applied: asked when the user closed it, [`ClosePolicy::ReturnToOrigin`]
        /// when its screen went ([`ScreenGone::DockBack`]), `None` when its dock emptied or
        /// went.
        policy: Option<ClosePolicy>,
        /// Each tab moved, and the dock it went to.
        moved: Vec<(Id, String)>,
    },
    /// The user moved or resized a float; reported once it stops changing.
    Moved {
        /// The float's key.
        key: String,
        /// Where it is now.
        place: Place,
    },
    /// A float was brought back onto a screen ([`Layout::repair_floats`]).
    Repaired {
        /// The float's key.
        key: String,
        /// The screen it was on, if it named one.
        from: Option<String>,
        /// The screen it is on now; `None` when it docked back (a
        /// [`Closed`](Self::Closed) follows).
        to: Option<String>,
    },
}

/// The floats' side of a [`DockSpace`], not saved.
#[derive(Clone, Debug, Default)]
pub(crate) struct Floating {
    /// Whether the space's floats are drawn ([`FloatsArea`]): a tab moved out of every dock
    /// then becomes a float, not a legacy window.
    pub(crate) enabled: bool,
    /// What hosts the floats made from a tab moved out.
    pub(crate) host: FloatHost,
    /// Tabs moved out, waiting for their float.
    pub(crate) pending: Vec<PendingFloat>,
    /// The layers of the floats over the main window, as last drawn.
    pub(crate) layers: Vec<LayerId>,
    /// The rects of the floats over the main window, as last drawn.
    pub(crate) rects: Vec<Rect>,
    /// The events not taken yet.
    pub(crate) events: Vec<FloatEvent>,
    /// Each native float's first builder: given again unchanged, so the window is not moved
    /// back where it opened.
    pub(crate) builders: HashMap<String, ViewportBuilder>,
    /// The floats whose place changed last frame.
    pub(crate) moving: BTreeSet<String>,
    /// The screens the floats were last repaired on.
    pub(crate) screens: Option<Vec<Screen>>,
    /// The last number given to a float's dock (`float-N`).
    pub(crate) next: u32,
}

/// A tab or a group moved out of a dock, waiting for its float.
#[derive(Clone, Debug)]
pub(crate) struct PendingFloat {
    /// The dock made for it, which names the float too.
    pub(crate) dock: String,
    /// Where it was released, in its viewport's points.
    pub(crate) rect: Rect,
    /// The same on the desktop, in physical pixels, when the system tells where its viewport
    /// is.
    pub(crate) physical: Option<Rect>,
    /// Its tabs.
    pub(crate) tabs: Vec<Id>,
}

/// Draws the floats of a [`Layout`] whose content is a dock of a [`DockSpace`]: over the main
/// window as `egui::Window`s ([`FloatHost::InWindow`]), or in native windows through a
/// [`ViewportHost`] ([`FloatHost::Native`]). Call it once a frame, in the main window's pass,
/// after the zones' docks.
///
/// Once a space's floats are drawn, a tab or a group moved out of every dock (released outside
/// them, ejected, or moved to a new window from its menu) becomes a float of the layout, in a
/// dock of its own, with its origin recorded ([`DockSpace::record_origin`]); legacy windows
/// ([`Surface::Window`]) become floats too. The float's host is the space's
/// [`float_host`](DockSpace::float_host).
///
/// Each frame it also:
///
/// - brings native floats back onto the host's screens when they change
///   ([`Layout::repair_floats`], with [`screen_gone`](Self::screen_gone));
/// - closes the floats whose dock emptied or went;
/// - keeps each float's place as the user moves and resizes it;
/// - asks what becomes of the tabs of a float the user closes
///   ([`TabViewer::close_policy`]) and applies it.
///
/// What happened is reported by [`DockSpace::take_float_events`].
///
/// ```
/// # use egui_stack::{DockArea, DockRules, DockSpace, Docked, FloatsArea, TabViewer};
/// # use egui_stack::layout::{Content, Layout, Zone};
/// # struct Tabs;
/// # impl TabViewer for Tabs {
/// #     type Tab = String;
/// #     fn id(&mut self, tab: &mut String) -> egui::Id { egui::Id::unique(tab.as_str()) }
/// #     fn title(&mut self, tab: &mut String) -> egui::WidgetText { tab.as_str().into() }
/// #     fn ui(&mut self, ui: &mut egui::Ui, tab: &mut String) { ui.label(tab.as_str()); }
/// # }
/// let mut space = DockSpace::new();
/// space.add_dock("centre", DockRules::default(), vec!["Editor".to_owned()]);
/// let mut docked = Docked::new(Layout::new(Zone::leaf("centre", Content::Dock("centre".into()))), space);
/// # egui::__run_test_ui(|ui| {
/// DockArea::dock(&mut docked.space, "centre").show_inside(ui, &mut Tabs);
/// docked.floats().show(ui.ctx(), &mut Tabs);
/// for event in docked.space.take_float_events() {
///     println!("{event:?}");
/// }
/// # });
/// ```
pub struct FloatsArea<'a, Tab> {
    layout: &'a mut Layout,
    space: &'a mut DockSpace<Tab>,
    host: Option<&'a mut dyn ViewportHost>,
    gone: ScreenGone,
    style: Option<Style>,
}

impl<Tab> Docked<Tab> {
    /// Draws the pair's floats: see [`FloatsArea`].
    pub fn floats(&mut self) -> FloatsArea<'_, Tab> {
        FloatsArea::new(&mut self.layout, &mut self.space)
    }
}

impl<'a, Tab> FloatsArea<'a, Tab> {
    /// The floats of `layout`, showing docks of `space`.
    pub fn new(layout: &'a mut Layout, space: &'a mut DockSpace<Tab>) -> Self {
        Self {
            layout,
            space,
            host: None,
            gone: ScreenGone::default(),
            style: None,
        }
    }

    /// Sets what hosts the native floats; by default [`EguiViewports`].
    #[must_use]
    pub fn host(mut self, host: &'a mut dyn ViewportHost) -> Self {
        self.host = Some(host);
        self
    }

    /// Sets what becomes of a native float whose screen is gone; by default
    /// [`ScreenGone::MoveOnto`].
    #[must_use]
    pub fn screen_gone(mut self, gone: ScreenGone) -> Self {
        self.gone = gone;
        self
    }

    /// Sets the docks' style in the floats; by default egui's.
    #[must_use]
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Draws the floats, back to front, and keeps the model up to date (see [`FloatsArea`]).
    pub fn show(self, ctx: &Context, viewer: &mut impl TabViewer<Tab = Tab>) {
        let Self {
            layout,
            space,
            host,
            gone,
            style,
        } = self;
        let mut default_host = EguiViewports;
        let host: &mut dyn ViewportHost = match host {
            Some(host) => host,
            None => &mut default_host,
        };
        space.floating.enabled = true;
        let screens = host.screens();

        legacy_windows(layout, space);
        open_pending(layout, space, &screens);
        sync_docks(layout, space);
        close_orphans(layout, space);
        repair(layout, space, viewer, &screens, gone);

        space.floating.layers.clear();
        space.floating.rects.clear();
        let floats: Vec<(String, String, FloatHost)> = layout
            .floats_back_to_front()
            .into_iter()
            .filter_map(|float| {
                Some((float.key.clone(), float.dock_name()?.to_owned(), float.host))
            })
            .collect();
        for (key, dock, kind) in floats {
            match kind {
                FloatHost::Native => {
                    show_native(
                        ctx, layout, space, host, viewer, &screens, &key, &dock, &style,
                    );
                }
                _ => show_in_window(ctx, layout, space, viewer, &key, &dock, &style),
            }
        }
    }
}

/// Legacy windows of the space ([`Surface::Window`]) become floats, hosted as the space says.
fn legacy_windows<Tab>(layout: &mut Layout, space: &mut DockSpace<Tab>) {
    let any = space
        .state()
        .iter_surfaces()
        .any(|surface| matches!(surface, Surface::Window(..)));
    if !any {
        return;
    }
    let host = space.floating.host;
    for mut float in space.windows_to_floats() {
        float.host = host;
        layout.open_float(float);
    }
}

/// The floats asked for by tabs moved out of a dock.
fn open_pending<Tab>(layout: &mut Layout, space: &mut DockSpace<Tab>, screens: &[Screen]) {
    for pending in std::mem::take(&mut space.floating.pending) {
        if !space.has_dock(&pending.dock) {
            continue;
        }
        let host = space.floating.host;
        let place = match host {
            FloatHost::Native => pending
                .physical
                .and_then(|rect| Place::from_physical(rect, screens))
                .unwrap_or_else(|| Place::new(pending.rect)),
            _ => Place::new(pending.rect),
        };
        let screen = place.screen.clone();
        layout.open_float(Float::dock(
            pending.dock.clone(),
            pending.dock.clone(),
            host,
            place,
        ));
        #[cfg(feature = "bus")]
        if let Some(bus) = &space.bus {
            let tabs = dock_keys(space, &pending.dock, bus.key);
            bus.emit::<crate::bus::WindowDetached>(crate::bus::DetachedInfo {
                key: pending.dock.clone(),
                dock: pending.dock.clone(),
                tabs,
                native: host == FloatHost::Native,
                screen: screen.clone(),
            });
            space.emit_change(float_change(crate::bus::Change::Detached, &pending.dock));
        }
        #[cfg(not(feature = "bus"))]
        let _ = screen;
        space.floating.events.push(FloatEvent::Detached {
            key: pending.dock.clone(),
            dock: pending.dock,
            tabs: pending.tabs,
            host,
        });
    }
}

/// Marks the docks the floats show.
fn sync_docks<Tab>(layout: &Layout, space: &mut DockSpace<Tab>) {
    for dock in space.docks_mut() {
        dock.floating = layout.float_of_dock(&dock.name).is_some();
    }
}

/// Closes the floats whose dock emptied (and was removed) or went.
fn close_orphans<Tab>(layout: &mut Layout, space: &mut DockSpace<Tab>) {
    let orphans: Vec<(String, String)> = layout
        .floats
        .iter()
        .filter_map(|float| {
            let dock = float.dock_name()?;
            let emptied = space.dock(dock).is_some_and(Tree::is_empty)
                && space
                    .rules(dock)
                    .is_some_and(|rules| !rules.keep_when_empty);
            let gone = !space.has_dock(dock) || emptied;
            gone.then(|| (float.key.clone(), dock.to_owned()))
        })
        .collect();
    for (key, dock) in orphans {
        layout.close_float(&key);
        // A dock emptied but kept goes with its float.
        if space.dock(&dock).is_some_and(Tree::is_empty) {
            space.remove_dock(&dock);
        }
        forget_native(space, &key);
        closed(space, key, dock, None, Vec::new(), Vec::new());
    }
}

/// Brings the native floats back onto `screens` when they changed since the last repair.
fn repair<Tab>(
    layout: &mut Layout,
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    screens: &[Screen],
    gone: ScreenGone,
) {
    if space.floating.screens.as_deref() == Some(screens) {
        return;
    }
    space.floating.screens = Some(screens.to_vec());
    let before: BTreeMap<String, Option<String>> = layout
        .floats
        .iter()
        .map(|float| (float.key.clone(), float.place.screen.clone()))
        .collect();
    let repair = layout.repair_floats(screens, gone);
    for key in repair.moved {
        let from = before.get(&key).cloned().flatten();
        let to = layout
            .float(&key)
            .and_then(|float| float.place.screen.clone());
        // A native window already open is told to move.
        space.floating.builders.remove(&key);
        repaired(space, key, from, to);
    }
    for float in repair.docked_back {
        let from = before.get(&float.key).cloned().flatten();
        repaired(space, float.key.clone(), from, None);
        forget_native(space, &float.key);
        if let Some(dock) = float.dock_name() {
            let dock = dock.to_owned();
            let (moved, keys) = put_back(space, viewer, &dock, &ClosePolicy::ReturnToOrigin);
            closed(
                space,
                float.key,
                dock,
                Some(ClosePolicy::ReturnToOrigin),
                moved,
                keys,
            );
        }
    }
}

/// A float brought back onto a screen, reported.
fn repaired<Tab>(
    space: &mut DockSpace<Tab>,
    key: String,
    from: Option<String>,
    to: Option<String>,
) {
    #[cfg(feature = "bus")]
    if let Some(bus) = &space.bus {
        bus.emit::<crate::bus::WindowRepaired>(crate::bus::RepairedInfo {
            key: key.clone(),
            from: from.clone(),
            to: to.clone(),
        });
        space.emit_change(float_change(crate::bus::Change::FloatRepaired, &key));
    }
    space
        .floating
        .events
        .push(FloatEvent::Repaired { key, from, to });
}

/// A float closed, reported. `keys` are the moved tabs' bus keys, when the space has a bus.
fn closed<Tab>(
    space: &mut DockSpace<Tab>,
    key: String,
    dock: String,
    policy: Option<ClosePolicy>,
    moved: Vec<(Id, String)>,
    keys: Vec<String>,
) {
    #[cfg(feature = "bus")]
    if let Some(bus) = &space.bus {
        bus.emit::<crate::bus::WindowClosed>(crate::bus::ClosedInfo {
            key: key.clone(),
            dock: dock.clone(),
            policy: policy.clone(),
            moved: keys
                .into_iter()
                .zip(moved.iter().map(|(_, dock)| dock.clone()))
                .collect(),
        });
        space.emit_change(float_change(crate::bus::Change::FloatClosed, &key));
    }
    #[cfg(not(feature = "bus"))]
    let _ = keys;
    space.floating.events.push(FloatEvent::Closed {
        key,
        dock,
        policy,
        moved,
    });
}

#[cfg(feature = "bus")]
fn float_change(change: crate::bus::Change, key: &str) -> crate::bus::LayoutChange {
    let mut change = crate::bus::LayoutChange::new(change);
    change.float = Some(key.to_owned());
    change
}

/// Forgets what was kept of a native float's window.
fn forget_native<Tab>(space: &mut DockSpace<Tab>, key: &str) {
    space.floating.builders.remove(key);
    space.floating.moving.remove(key);
}

/// The float `key`'s place changed to `place` (or not): kept, and reported once it stops
/// changing.
fn track_place<Tab>(layout: &mut Layout, space: &mut DockSpace<Tab>, key: &str, place: Place) {
    let Some(float) = layout.float_mut(key) else {
        return;
    };
    if float.place != place {
        float.place = place;
        space.floating.moving.insert(key.to_owned());
    } else if space.floating.moving.remove(key) {
        let place = float.place.clone();
        #[cfg(feature = "bus")]
        space.emit_change(float_change(crate::bus::Change::FloatMoved, key));
        space.floating.events.push(FloatEvent::Moved {
            key: key.to_owned(),
            place,
        });
    }
}

/// The title of the dock `dock`'s window: its focused (or first) leaf's active tab.
fn dock_title<Tab>(
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    dock: &str,
) -> egui::WidgetText {
    let Some(tree) = space.dock_mut(dock) else {
        return dock.into();
    };
    let node = tree
        .focused_leaf()
        .filter(|&node| tree.leaf(node).is_ok_and(|leaf| !leaf.tabs.is_empty()))
        .or_else(|| {
            tree.breadth_first_index_iter()
                .find(|&node| tree.leaf(node).is_ok_and(|leaf| !leaf.tabs.is_empty()))
        });
    match node.and_then(|node| tree.leaf_mut(node).ok()) {
        Some(leaf) => {
            let active = leaf.active.0.min(leaf.tabs.len() - 1);
            viewer.title(&mut leaf.tabs[active])
        }
        None => dock.into(),
    }
}

/// A float over the main window, as an `egui::Window`.
fn show_in_window<Tab>(
    ctx: &Context,
    layout: &mut Layout,
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    key: &str,
    dock: &str,
    style: &Option<Style>,
) {
    let Some(place) = layout.float(key).map(|float| float.place.clone()) else {
        return;
    };
    let title = dock_title(space, viewer, dock);
    let mut open = true;
    let id = space.id().with(("egui_stack.float", key));
    let shown = egui::Window::new(title)
        .id(id)
        .default_pos(place.rect.min)
        .default_size(place.rect.size())
        .min_size(vec2(120.0, 80.0))
        .collapsible(false)
        .resizable(true)
        .open(&mut open)
        .show(ctx, |ui| {
            let mut area = DockArea::dock(space, dock);
            if let Some(style) = style {
                area = area.style(style.clone());
            }
            area.show_inside(ui, viewer);
        });
    if let Some(shown) = shown {
        let rect = shown.response.rect;
        space.floating.layers.push(shown.response.layer_id);
        space.floating.rects.push(rect);
        if shown.response.contains_pointer() && ctx.input(|i| i.pointer.any_pressed()) {
            layout.raise_float(key);
        }
        let mut moved = place.clone();
        moved.rect = rect;
        track_place(layout, space, key, moved);
    }
    if !open {
        let policy = ask_close(space, viewer, key, dock);
        if policy != ClosePolicy::Keep {
            close_with(layout, space, viewer, key, policy);
        }
    }
}

/// A native float, through the host.
#[allow(clippy::too_many_arguments)]
fn show_native<Tab>(
    ctx: &Context,
    layout: &mut Layout,
    space: &mut DockSpace<Tab>,
    host: &mut dyn ViewportHost,
    viewer: &mut impl TabViewer<Tab = Tab>,
    screens: &[Screen],
    key: &str,
    dock: &str,
    style: &Option<Style>,
) {
    let Some(float) = layout.float(key).cloned() else {
        return;
    };
    let id = host.viewport_id(key);
    let builder = match space.floating.builders.get(key) {
        Some(builder) => builder.clone(),
        None => {
            let builder = first_builder(host.builder(&float), &float.place, screens);
            space
                .floating
                .builders
                .insert(key.to_owned(), builder.clone());
            builder
        }
    };
    let area_id = space.id().with(("egui_stack.float", key));
    let mut closing = None;
    let mut seen: Option<(egui::Vec2, Option<bool>, Option<bool>)> = None;
    host.show(ctx, id, builder, &mut |ui: &mut Ui| {
        let (close, maximized, fullscreen) = ui.ctx().input(|i| {
            let info = i.viewport();
            (info.close_requested(), info.maximized, info.fullscreen)
        });
        seen = Some((ui.ctx().content_rect().size(), maximized, fullscreen));
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            let mut area = DockArea::dock(space, dock).id(area_id).in_native_float();
            if let Some(style) = style {
                area = area.style(style.clone());
            }
            area.show_inside(ui, viewer);
        });
        if close {
            let policy = ask_close(space, viewer, key, dock);
            if policy == ClosePolicy::Keep {
                ui.ctx().send_viewport_cmd(ViewportCommand::CancelClose);
            } else {
                closing = Some(policy);
            }
        }
    });
    if let Some(policy) = closing {
        close_with(layout, space, viewer, key, policy);
        return;
    }
    let mut place = float.place.clone();
    if let Some(physical) = host.outer_rect(ctx, id)
        && let Some(found) = Place::from_physical(physical, screens)
    {
        place.screen = found.screen;
        place.rect = found.rect;
        place.scale = found.scale;
    } else if let Some((size, _, _)) = seen {
        // Where positions are hidden, the size is all the system tells.
        place.rect = Rect::from_min_size(place.rect.min, size);
    }
    if let Some((_, maximized, fullscreen)) = seen {
        place.maximized = maximized.unwrap_or(place.maximized);
        place.fullscreen = fullscreen.unwrap_or(place.fullscreen);
    }
    track_place(layout, space, key, place);
}

/// The builder a native float's window opens with: the host's, with the place's position (on
/// a known screen), size and states where the host left them unset.
fn first_builder(
    mut builder: ViewportBuilder,
    place: &Place,
    screens: &[Screen],
) -> ViewportBuilder {
    if builder.position.is_none()
        && let Some(screen) = place
            .screen
            .as_deref()
            .and_then(|name| screens.iter().find(|screen| screen.name == name))
    {
        let physical = place.physical(screen);
        builder = builder.with_position((physical.min.to_vec2() / screen.scale).to_pos2());
    }
    if builder.inner_size.is_none() {
        builder = builder.with_inner_size(place.rect.size());
    }
    if builder.maximized.is_none() && place.maximized {
        builder = builder.with_maximized(true);
    }
    if builder.fullscreen.is_none() && place.fullscreen {
        builder = builder.with_fullscreen(true);
    }
    builder
}

/// The tabs of the dock `dock`, by the viewer's ids, in order.
fn dock_ids<Tab>(
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    dock: &str,
) -> Vec<Id> {
    let Some(tree) = space.dock_mut(dock) else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    for node in tree.iter_mut() {
        if let Some(leaf) = node.get_leaf_mut() {
            ids.extend(leaf.tabs.iter_mut().map(|tab| viewer.id(tab)));
        }
    }
    ids
}

/// The tabs of the dock `dock`, by the bus's key function, in order.
#[cfg(feature = "bus")]
fn dock_keys<Tab>(space: &DockSpace<Tab>, dock: &str, key: fn(&Tab) -> String) -> Vec<String> {
    space
        .dock(dock)
        .map(|tree| {
            tree.iter()
                .filter_map(|node| node.tabs())
                .flatten()
                .map(key)
                .collect()
        })
        .unwrap_or_default()
}

/// What becomes of the tabs of the float `key` the user closes: the viewer's answer, then the
/// bus's.
fn ask_close<Tab>(
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    key: &str,
    dock: &str,
) -> ClosePolicy {
    let tabs = dock_ids(space, viewer, dock);
    let close = FloatClose {
        key: key.to_owned(),
        dock: dock.to_owned(),
        tabs,
    };
    let policy = viewer.close_policy(&close);
    #[cfg(feature = "bus")]
    if let Some(bus) = &space.bus {
        let origins = close
            .tabs
            .iter()
            .map(|&tab| space.origin(tab).map(|origin| origin.dock.clone()))
            .collect();
        let ask = crate::bus::CloseAsk {
            key: key.to_owned(),
            dock: dock.to_owned(),
            tabs: dock_keys(space, dock, bus.key),
            origins,
        };
        return bus.close_policy(ask, policy);
    }
    policy
}

/// Closes the float `key` with `policy` (not [`ClosePolicy::Keep`]), and reports it.
fn close_with<Tab>(
    layout: &mut Layout,
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    key: &str,
    policy: ClosePolicy,
) {
    let Some(float) = layout.close_float(key) else {
        return;
    };
    forget_native(space, key);
    let Some(dock) = float.dock_name().map(str::to_owned) else {
        return;
    };
    let (moved, keys) = put_back(space, viewer, &dock, &policy);
    closed(space, key.to_owned(), dock, Some(policy), moved, keys);
    sync_docks(layout, space);
}

/// Empties the dock `dock` (and removes it) as `policy` says: its tabs go back to their
/// origins, into a dock, or close. Returns each tab moved with the dock it went to, and the
/// moved tabs' bus keys.
fn put_back<Tab>(
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    dock: &str,
    policy: &ClosePolicy,
) -> (Vec<(Id, String)>, Vec<String>) {
    let Some(mut tree) = space.remove_dock(dock) else {
        return (Vec::new(), Vec::new());
    };
    let mut tabs: Vec<Tab> = Vec::new();
    for node in tree.iter_mut() {
        if let Some(leaf) = node.get_leaf_mut() {
            tabs.append(&mut leaf.tabs);
        }
    }
    let mut moved = Vec::new();
    #[cfg_attr(not(feature = "bus"), allow(unused_mut))]
    let mut keys = Vec::new();
    for mut tab in tabs {
        let id = viewer.id(&mut tab);
        let origin = space.take_origin(id);
        #[cfg(feature = "bus")]
        let key = space.bus.as_ref().map(|bus| (bus.key)(&tab));
        let into = match policy {
            ClosePolicy::CloseTabs | ClosePolicy::Keep => continue,
            ClosePolicy::Into(name) if space.has_dock(name) => Some((name.clone(), None)),
            _ => origin
                .filter(|origin| space.has_dock(&origin.dock))
                .map(|origin| (origin.dock.clone(), Some((origin.before, origin.after)))),
        };
        let fallback = || {
            space
                .docks()
                .iter()
                .find(|dock| !dock.floating)
                .map(|dock| (dock.name.clone(), None))
        };
        let Some((target, near)) = into.or_else(fallback) else {
            continue;
        };
        put_tab(space, viewer, &target, tab, near);
        moved.push((id, target));
        #[cfg(feature = "bus")]
        keys.extend(key);
    }
    (moved, keys)
}

/// Puts `tab` in the dock `dock`: after the tab it was next to (`near`: the tabs before and
/// after it), else in the dock's focused (or first) leaf, else as the empty dock's only tab.
fn put_tab<Tab>(
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    dock: &str,
    tab: Tab,
    near: Option<(Option<Id>, Option<Id>)>,
) {
    let Some(tree) = space.dock_mut(dock) else {
        return;
    };
    if tree.is_empty() {
        *tree = Tree::new(vec![tab]);
        return;
    }
    if let Some((before, after)) = near {
        let nodes: Vec<_> = tree.breadth_first_index_iter().collect();
        for node in nodes {
            let Ok(leaf) = tree.leaf_mut(node) else {
                continue;
            };
            let ids: Vec<Id> = leaf.tabs.iter_mut().map(|tab| viewer.id(tab)).collect();
            let at = before
                .and_then(|before| ids.iter().position(|&id| id == before))
                .map(|i| i + 1)
                .or_else(|| after.and_then(|after| ids.iter().position(|&id| id == after)));
            if let Some(at) = at {
                leaf.insert_tab(TabIndex(at), tab);
                return;
            }
        }
    }
    let leaf = tree
        .focused_leaf()
        .filter(|&node| tree.leaf(node).is_ok())
        .or_else(|| {
            tree.breadth_first_index_iter()
                .find(|&node| tree.leaf(node).is_ok())
        });
    match leaf {
        Some(node) => tree[node].append_tab(tab),
        None => *tree = Tree::new(vec![tab]),
    }
}
