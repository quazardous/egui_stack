//! Floats drawn: the docks of a [`DockSpace`] the layout's [`Float`]s show, over the main
//! window ([`FloatHost::InWindow`]) or in native windows the application hosts
//! ([`FloatHost::Native`], through a [`ViewportHost`]). See [`FloatsArea`].

use std::collections::{BTreeMap, BTreeSet, HashMap};

use egui::{
    Context, Id, LayerId, Modifiers, Rect, Ui, Vec2, ViewportBuilder, ViewportCommand, ViewportId,
    WindowDrag, vec2,
};

use crate::layout::{Float, FloatHost, Layout, Place, Screen, ScreenGone};
use crate::{
    DockArea, DockPainter, DockSpace, Docked, GroupHandles, NodePath, Style, Surface, TabViewer,
    Tree,
};

mod gestures;
mod work_area;

pub use gestures::FloatMove;
pub(crate) use gestures::{WindowMove, move_handle_id};
pub(crate) use work_area::BoundZones;
pub use work_area::{Outside, WorkArea};

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

    /// Whether this host opens native windows: by default, unless egui embeds its viewports
    /// ([`Context::embed_viewports`]: on the web, or a backend with a single viewport). A host
    /// that draws the floats embedded says `false`. Without native windows,
    /// [`Outside::Native`] falls back to [`Outside::Cancel`].
    fn native_windows(&self, ctx: &Context) -> bool {
        !ctx.embed_viewports()
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
    /// Where each float's window was last seen (its place's corner): a place the application
    /// changed since then moves the window there.
    pub(crate) placed: HashMap<String, egui::Pos2>,
    /// The screens the floats were last repaired on.
    pub(crate) screens: Option<Vec<Screen>>,
    /// The last number given to a float's dock (`float-N`).
    pub(crate) next: u32,
    /// The drag in progress across the space's viewports.
    pub(crate) session: Option<crate::dock_area::session::DragSession>,
    /// The viewport of each native float drawn, by key.
    pub(crate) viewports: HashMap<String, ViewportId>,
    /// The float whose window is being moved with [`FloatMove::Dock`].
    pub(crate) window_move: Option<WindowMove>,
    /// The height a float over the main window takes around its dock (title bar, frame), by
    /// key, as last drawn.
    pub(crate) chrome: HashMap<String, f32>,
    /// The floats over the main window drawn last with all their groups collapsed.
    pub(crate) collapsed: BTreeSet<String>,
    /// The docks of the floats whose close window button was clicked this frame
    /// ([`FloatsArea::title_bar`]), closed once their window is drawn.
    pub(crate) close_asked: BTreeSet<String>,
    /// Where the floats over the main window may go ([`FloatsArea::work_area`]), as last set.
    pub(crate) work_area: WorkArea,
    /// The layout's zones last read by [`DockSpace::bind`], for the work area's regions.
    pub(crate) zones: BoundZones,
    /// Where each float just opened from a release over the main window was released: its
    /// first region is the one there, whatever its size overlaps.
    pub(crate) released_at: HashMap<String, egui::Pos2>,
    /// Whether the host opens native windows ([`ViewportHost::native_windows`]), as last told.
    pub(crate) native_windows: bool,
    /// Where the pointer held each float over the main window being moved, from its corner,
    /// while it was over the window: a float torn out ([`Outside::Native`]) opens with the
    /// pointer there.
    pub(crate) grabs: HashMap<String, egui::Vec2>,
}

impl Floating {
    /// The work area's regions this frame, in the main window's points; `None` when the floats
    /// are free. `docks` are the space's docks, for the space drawn without
    /// [`DockSpace::bind`].
    pub(crate) fn regions(&self, docks: &[crate::dock_space::Dock]) -> Option<Vec<Rect>> {
        let drawn = docks
            .iter()
            .filter(|dock| !dock.floating && work_area::main_window(dock.viewport))
            .filter_map(|dock| dock.rect);
        self.work_area.regions(&self.zones, drawn)
    }

    /// What hosts the float made from a tab or a group released over nothing at `pointer`, in
    /// the main window's points: the space's host inside the work area; past its edge, a
    /// native window with [`Outside::Native`] where native windows are available; `None` where
    /// the release is refused ([`Outside::Cancel`]).
    pub(crate) fn release_host(
        &self,
        docks: &[crate::dock_space::Dock],
        pointer: egui::Pos2,
    ) -> Option<FloatHost> {
        let Some(regions) = self.regions(docks) else {
            return Some(self.host);
        };
        if regions.iter().any(|region| region.contains(pointer)) {
            return Some(self.host);
        }
        match self.work_area.outside_rule() {
            Outside::Native if self.native_windows => Some(FloatHost::Native),
            Outside::Native | Outside::Cancel => None,
        }
    }

    /// The viewport `viewport` closed (a float's window): a drag that began there ends, and one
    /// over it is cancelled (nothing moves at its release).
    pub(crate) fn viewport_closed(&mut self, viewport: ViewportId) {
        let Some(session) = &mut self.session else {
            return;
        };
        if session.source == viewport {
            self.session = None;
        } else if let Some(report) = session.reports.remove(&viewport)
            && report.stamp + 1 >= session.pass
        {
            session.cancelled = true;
            session.reports.clear();
        }
    }
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
    /// What hosts it, chosen at its release: the space's host, or a native window for one
    /// torn out of the work area ([`Outside::Native`]).
    pub(crate) host: FloatHost,
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
/// - shrinks a float over the main window whose groups are all collapsed to their tab bars,
///   without its resize grip, its place keeping the height it gets back once a group is
///   expanded. A native float keeps its window's size: the system owns the window, whose
///   frame and resizing are its own, and a collapsed group leaves its body to the other
///   groups, as in a dock;
/// - asks what becomes of the tabs of a float the user closes
///   ([`TabViewer::close_policy`]) and applies it.
///
/// What happened is reported by [`DockSpace::take_float_events`].
///
/// The application shapes the floats' docks: its [`painter`](Self::painter) paints them (tab
/// bars, outlines), [`configure`](Self::configure) sets each float's [`DockArea`] options (no
/// splits, no close or add buttons…), and [`decorate`](Self::decorate) draws over each float
/// once its dock is drawn, knowing where its tab bars are ([`FloatShown`]).
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
    handles: GroupHandles,
    modifiers: Modifiers,
    float_move: FloatMove,
    title_bar: bool,
    work_area: WorkArea,
    add_buttons: bool,
    painter: Option<&'a (dyn DockPainter + Sync)>,
    configure: Option<Configure<'a, Tab>>,
    decorate: Option<Decorate<'a>>,
}

/// A float's [`DockArea`] shaped by the application ([`FloatsArea::configure`]).
type Configure<'a, Tab> =
    Box<dyn for<'d> FnMut(&Float, DockArea<'d, Tab>) -> DockArea<'d, Tab> + 'a>;

/// What the application draws over a float ([`FloatsArea::decorate`]).
type Decorate<'a> = Box<dyn FnMut(&mut Ui, &FloatShown) + 'a>;

/// A float just drawn, for [`FloatsArea::decorate`].
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FloatShown {
    /// The float's key.
    pub key: String,
    /// The dock it shows.
    pub dock: String,
    /// The viewport it is drawn in: its native window's ([`ViewportHost::viewport_id`]), or the
    /// main window's for a float over it.
    pub viewport: ViewportId,
    /// Whether it is a native float ([`FloatHost::Native`]).
    pub native: bool,
    /// The rect its dock was drawn in, in the viewport's points.
    pub rect: Rect,
    /// Its tab bars, each with its leaf, in the viewport's points: the whole bar, margins
    /// included, as [`DockArea::tab_bars`] lists them. A hidden tab bar is not listed.
    pub tab_bars: Vec<(NodePath, Rect)>,
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
            handles: GroupHandles::ALL,
            modifiers: Modifiers::SHIFT,
            float_move: FloatMove::Move,
            title_bar: false,
            work_area: WorkArea::window(),
            add_buttons: false,
            painter: None,
            configure: None,
            decorate: None,
        }
    }

    /// Sets what paints the floats' docks ([`DockArea::painter`]): their tab bars and tabs,
    /// and, while a tab is dragged out of another window where window positions are hidden,
    /// their [`window_outline`](DockPainter::window_outline). By default the area's own
    /// ([`StylePainter`](crate::StylePainter)).
    #[must_use]
    pub fn painter(mut self, painter: &'a (dyn DockPainter + Sync)) -> Self {
        self.painter = Some(painter);
        self
    }

    /// Shapes each float's [`DockArea`] before it is drawn: `configure` gets the float and the
    /// area, with egui_stack's settings applied (style, painter, handles, modifiers), and
    /// returns it, its own options set; for example
    /// `area.allowed_splits(AllowedSplits::None).show_close_buttons(false)`.
    ///
    /// Leave the area's [`id`](DockArea::id) alone: the floats' drags and move handles are
    /// keyed by it.
    #[must_use]
    pub fn configure(
        mut self,
        configure: impl for<'d> FnMut(&Float, DockArea<'d, Tab>) -> DockArea<'d, Tab> + 'a,
    ) -> Self {
        self.configure = Some(Box::new(configure));
        self
    }

    /// Draws over each float once its dock is drawn, in the same pass and the float's own
    /// [`Ui`] (its native window's, or the `egui::Window`'s): `decorate` gets that `Ui` and
    /// where the float's tab bars are ([`FloatShown::tab_bars`]), for a button of the
    /// application at the end of a bar, say. What it adds is drawn and hit above the dock.
    #[must_use]
    pub fn decorate(mut self, decorate: impl FnMut(&mut Ui, &FloatShown) + 'a) -> Self {
        self.decorate = Some(Box::new(decorate));
        self
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

    /// Sets the handles of the floats' docks ([`DockArea::group_handles`]), the window's move
    /// handle ([`GroupHandles::MOVE_WINDOW`]) included; by default [`GroupHandles::ALL`].
    #[must_use]
    pub fn group_handles(mut self, handles: GroupHandles) -> Self {
        self.handles = handles;
        self
    }

    /// Sets the modifiers that make a tab's drag carry its group in the floats' docks
    /// ([`DockArea::group_drag_modifiers`]); by default [`Modifiers::SHIFT`].
    #[must_use]
    pub fn group_drag_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    /// Sets what moving a float's window does, by its title bar or its move handle; by
    /// default [`FloatMove::Move`]: it only moves, and never docks.
    #[must_use]
    pub fn float_move(mut self, float_move: FloatMove) -> Self {
        self.float_move = float_move;
        self
    }

    /// Sets whether a float over the main window ([`FloatHost::InWindow`]) has a title bar of
    /// its own, with its title and a close button, above its dock; by default it has none: its
    /// top tab bar is its top, the window's move handle ([`GroupHandles::MOVE_WINDOW`]) moves
    /// it and a close window button beside the handle closes it, as the title bar's close
    /// button does: it asks [`TabViewer::close_policy`] what becomes of the tabs, whether or
    /// not they can be closed ([`TabViewer::is_closeable`]). The tab bars' close all buttons
    /// close tabs, not the window. Without the move handle (turned off in
    /// [`group_handles`](Self::group_handles)), the title bar is kept, so that the window can
    /// still be moved. A native float keeps the system's decoration either way.
    #[must_use]
    pub fn title_bar(mut self, title_bar: bool) -> Self {
        self.title_bar = title_bar;
        self
    }

    /// Shows an add button in each tab bar of the floats' docks
    /// ([`DockArea::show_add_buttons`]), as the application's docks show theirs: give it the
    /// value given to the docks, so that a group shows its add button whether it is docked or
    /// floating. A click calls [`TabViewer::on_add`] with the group's path, the float's surface
    /// in it. By default `false`, as for a [`DockArea`]. A float's
    /// [`configure`](Self::configure) can still change it.
    #[must_use]
    pub fn show_add_buttons(mut self, show_add_buttons: bool) -> Self {
        self.add_buttons = show_add_buttons;
        self
    }

    /// Sets where the floats over the main window may go, and where a tab dragged out of every
    /// dock may become one: see [`WorkArea`]. By default [`WorkArea::window`], the whole window.
    /// Native floats ignore it.
    ///
    /// Give it every frame, as the other options: the space keeps it (not saved) for its docks,
    /// which decide a drop as they are drawn, before the floats. On the very first frame the
    /// docks have not read it yet, which no drag notices: a drag takes several frames.
    #[must_use]
    pub fn work_area(mut self, work_area: WorkArea) -> Self {
        self.work_area = work_area;
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
            handles,
            modifiers,
            float_move,
            title_bar,
            work_area,
            add_buttons,
            painter,
            configure,
            decorate,
        } = self;
        let mut gestures = Gestures {
            handles,
            modifiers,
            float_move,
            title_bar,
            add_buttons,
            painter,
            configure,
            decorate,
        };
        let mut default_host = EguiViewports;
        let host: &mut dyn ViewportHost = match host {
            Some(host) => host,
            None => &mut default_host,
        };
        space.floating.enabled = true;
        space.floating.work_area = work_area;
        space.floating.native_windows = host.native_windows(ctx);
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
                        ctx,
                        layout,
                        space,
                        host,
                        viewer,
                        &screens,
                        &key,
                        &dock,
                        &style,
                        &mut gestures,
                    );
                }
                _ => show_in_window(
                    ctx,
                    layout,
                    space,
                    viewer,
                    &key,
                    &dock,
                    &style,
                    &mut gestures,
                ),
            }
        }
    }
}

/// How the floats' docks and windows answer the user's gestures, and what the application
/// sets of them.
struct Gestures<'a, Tab> {
    handles: GroupHandles,
    modifiers: Modifiers,
    float_move: FloatMove,
    /// [`FloatsArea::title_bar`].
    title_bar: bool,
    /// [`FloatsArea::show_add_buttons`].
    add_buttons: bool,
    painter: Option<&'a (dyn DockPainter + Sync)>,
    configure: Option<Configure<'a, Tab>>,
    decorate: Option<Decorate<'a>>,
}

impl<'a, Tab> Gestures<'a, Tab> {
    /// The area drawing the dock of `float`, with these gestures, `style`, the application's
    /// painter and options.
    fn area<'d>(
        &mut self,
        float: &Float,
        area: DockArea<'d, Tab>,
        style: &Option<Style>,
    ) -> DockArea<'d, Tab>
    where
        'a: 'd,
    {
        let mut area = area
            .group_handles(self.handles)
            .group_drag_modifiers(self.modifiers)
            .show_add_buttons(self.add_buttons);
        if let Some(style) = style {
            area = area.style(style.clone());
        }
        if let Some(painter) = self.painter {
            area = area.painter(painter);
        }
        match &mut self.configure {
            Some(configure) => configure(float, area),
            None => area,
        }
    }

    /// Hands the float `key` (dock `dock`), just drawn by the area `area_id` in `ui`, to the
    /// application's [`FloatsArea::decorate`].
    #[allow(clippy::too_many_arguments)]
    fn decorate(
        &mut self,
        ui: &mut Ui,
        space: &DockSpace<Tab>,
        area_id: Id,
        key: &str,
        dock: &str,
        native: bool,
        rect: Rect,
    ) {
        let Some(decorate) = &mut self.decorate else {
            return;
        };
        let surface = space.dock_surface(dock);
        let tab_bars = DockArea::tab_bars(ui.ctx(), area_id)
            .into_iter()
            .filter(|(path, _)| Some(path.surface) == surface)
            .collect();
        let shown = FloatShown {
            key: key.to_owned(),
            dock: dock.to_owned(),
            viewport: ui.ctx().viewport_id(),
            native,
            rect,
            tab_bars,
        };
        decorate(ui, &shown);
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
        let host = pending.host;
        let place = match host {
            FloatHost::Native => pending
                .physical
                .and_then(|rect| Place::from_physical(rect, screens))
                .unwrap_or_else(|| Place::new(pending.rect)),
            _ => Place::new(pending.rect),
        };
        let screen = place.screen.clone();
        if host != FloatHost::Native {
            space
                .floating
                .released_at
                .insert(pending.dock.clone(), pending.rect.min);
        }
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

/// Forgets what was kept of a native float's window, closed: a drag that began in it ends, and
/// one over it is cancelled.
fn forget_native<Tab>(space: &mut DockSpace<Tab>, key: &str) {
    space.floating.released_at.remove(key);
    space.floating.chrome.remove(key);
    space.floating.collapsed.remove(key);
    space.floating.builders.remove(key);
    space.floating.moving.remove(key);
    space.floating.placed.remove(key);
    if space
        .floating
        .window_move
        .as_ref()
        .is_some_and(|moving| moving.key == key)
    {
        space.floating.window_move = None;
    }
    if let Some(viewport) = space.floating.viewports.remove(key) {
        space.floating.viewport_closed(viewport);
    }
}

/// The float `key`'s place changed to `place` (or not): kept, and reported once it stops
/// changing.
fn track_place<Tab>(layout: &mut Layout, space: &mut DockSpace<Tab>, key: &str, place: Place) {
    let Some(float) = layout.float_mut(key) else {
        return;
    };
    space.floating.placed.insert(key.to_owned(), place.rect.min);
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

/// The title of the dock `dock`'s window: its focused (or first) leaf's active tab, else the
/// dock's name.
fn dock_title<Tab>(
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    dock: &str,
) -> egui::WidgetText {
    space
        .dock_mut(dock)
        .and_then(|tree| tree_title(tree, viewer))
        .unwrap_or_else(|| dock.into())
}

/// The title of a float's window showing `tree`: its focused (or first) leaf's active tab;
/// `None` when the tree has no tab.
pub(crate) fn tree_title<Tab>(
    tree: &mut Tree<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
) -> Option<egui::WidgetText> {
    let node = tree
        .focused_leaf()
        .filter(|&node| tree.leaf(node).is_ok_and(|leaf| !leaf.tabs.is_empty()))
        .or_else(|| {
            tree.breadth_first_index_iter()
                .find(|&node| tree.leaf(node).is_ok_and(|leaf| !leaf.tabs.is_empty()))
        })?;
    let leaf = tree.leaf_mut(node).ok()?;
    let active = leaf.active.0.min(leaf.tabs.len() - 1);
    Some(viewer.title(&mut leaf.tabs[active]))
}

/// A float over the main window, as an `egui::Window`: moved by its move handle, or by its
/// title bar when it has one ([`FloatsArea::title_bar`]), never by the empty part of a tab
/// bar.
#[allow(clippy::too_many_arguments)]
fn show_in_window<Tab>(
    ctx: &Context,
    layout: &mut Layout,
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    key: &str,
    dock: &str,
    style: &Option<Style>,
    gestures: &mut Gestures<'_, Tab>,
) {
    let Some(float) = layout.float(key).cloned() else {
        return;
    };
    let place = float.place.clone();
    let app_moved = moved_by_app(space, &float);
    let title = dock_title(space, viewer, dock);
    let mut open = true;
    let id = space.id().with(("egui_stack.float", key));
    let handle = move_handle_id(space.id(), dock);
    // Without a title bar, only the move handle moves the window: its body and the empty
    // part of its tab bars keep their own drags.
    let title_bar = gestures.title_bar || !gestures.handles.contains(GroupHandles::MOVE_WINDOW);
    // All its groups collapsed, the float is as tall as their tab bars and is not resized: its
    // place keeps the height it had, given back once a group is expanded.
    let mut probe = DockArea::dock(space, dock).show_add_buttons(gestures.add_buttons);
    if let Some(style) = style {
        probe = probe.style(style.clone());
    }
    let collapsed_height = probe.collapsed_dock_height(&ctx.global_style());
    let chrome = space.floating.chrome.get(key).copied().unwrap_or_else(|| {
        egui::Frame::window(&ctx.global_style())
            .total_margin()
            .sum()
            .y
            + if title_bar {
                ctx.global_style().spacing.interact_size.y
            } else {
                0.0
            }
    });
    let was_collapsed = space.floating.collapsed.contains(key);
    let window = egui::Window::new(title)
        .id(id)
        .default_pos(place.rect.min)
        .default_size(place.rect.size())
        .min_size(vec2(120.0, 80.0))
        .collapsible(false)
        .resizable(true)
        .title_bar(title_bar)
        .drag_area(if title_bar {
            WindowDrag::TitleBar
        } else {
            WindowDrag::Off
        })
        .open(&mut open);
    // The application changed the float's place: the window goes there.
    let window = if app_moved {
        window.current_pos(place.rect.min)
    } else {
        window
    };
    // The work area: the window stays in one region, moved by the pointer from one to another,
    // and is no larger than it. egui's `constrain_to` bounds the position; the size is capped
    // too (egui 0.37 also caps a window's largest size to its constrain rect). A window the
    // region moves or shrinks is kept and reported below, as one the user moved.
    let released_at = space.floating.released_at.remove(key);
    let region = space.floating.regions(space.docks()).and_then(|regions| {
        let moved_by_user = [id.with("__title_click"), handle]
            .into_iter()
            .any(|id| ctx.read_response(id).is_some_and(|r| r.dragged()));
        let pointer = if moved_by_user {
            ctx.input(|i| i.pointer.latest_pos())
        } else {
            released_at
        };
        work_area::region_of(&regions, place.rect, pointer)
    });
    let window = match region {
        Some(region) => window.constrain_to(region).max_size(region.size()),
        None => window,
    };
    let window = match collapsed_height {
        Some(height) => window.fixed_size(vec2(place.rect.width(), height + chrome)),
        // Expanded again: the height it had, which egui keeps from the next frame on.
        None if was_collapsed => window
            .min_height(place.rect.height())
            .max_height(place.rect.height()),
        None => window,
    };
    if collapsed_height.is_some() {
        space.floating.collapsed.insert(key.to_owned());
    } else {
        space.floating.collapsed.remove(key);
    }
    // The move handle dragged in the last pass: the window follows the pointer, moved as egui
    // moves it by its title bar (a `current_pos` would be undone by the title-bar-only drag).
    if ctx.read_response(handle).is_some_and(|r| r.dragged()) {
        let delta = ctx.input(|i| i.pointer.delta());
        if delta != egui::Vec2::ZERO
            && let Some(mut state) = egui::AreaState::load(ctx, id)
            && let Some(pivot) = state.pivot_pos.as_mut()
        {
            *pivot += delta;
            ctx.memory_mut(|mem| {
                mem.areas_mut()
                    .set_state(LayerId::new(egui::Order::Middle, id), state);
            });
        }
    }
    let mut content = None;
    let shown = window.show(ctx, |ui| {
        let area_id = space.id();
        let rect = ui.available_rect_before_wrap();
        content = Some(rect);
        let mut area = DockArea::dock(space, dock);
        if !title_bar {
            // Its top tab bar is its top: the close button goes there, by the move handle.
            area = area.float_close_button();
        }
        gestures.area(&float, area, style).show_inside(ui, viewer);
        gestures.decorate(ui, space, area_id, key, dock, false, rect);
    });
    if gestures.float_move == FloatMove::Dock {
        // egui's title bar of a window: its drag moves the window.
        let title_bar = ctx.read_response(id.with("__title_click"));
        let handle = ctx.read_response(handle);
        let moving = [&title_bar, &handle]
            .iter()
            .any(|r| r.as_ref().is_some_and(|r| r.dragged()));
        let released = [&title_bar, &handle]
            .iter()
            .any(|r| r.as_ref().is_some_and(|r| r.drag_stopped()));
        let ours = space
            .floating
            .window_move
            .as_ref()
            .is_some_and(|moving| moving.key == key);
        if moving || (released && ours) {
            let pointer = ctx.input(|i| i.pointer.latest_pos());
            gestures::track(
                ctx, layout, space, viewer, style, key, dock, pointer, released,
            );
            if layout.float(key).is_none() {
                // Docked.
                return;
            }
        }
    }
    if let Some(shown) = &shown
        && tear_out(
            ctx,
            layout,
            space,
            key,
            [id.with("__title_click"), handle],
            shown.response.rect,
        )
    {
        return;
    }
    if let Some(shown) = shown {
        let rect = shown.response.rect;
        space.floating.layers.push(shown.response.layer_id);
        space.floating.rects.push(rect);
        if let Some(content) = content {
            space
                .floating
                .chrome
                .insert(key.to_owned(), rect.height() - content.height());
        }
        if shown.response.contains_pointer() && ctx.input(|i| i.pointer.any_pressed()) {
            layout.raise_float(key);
        }
        let mut moved = place.clone();
        moved.rect = if collapsed_height.is_some() {
            // Collapsed, it keeps the height to expand to.
            Rect::from_min_size(rect.min, vec2(rect.width(), place.rect.height()))
        } else {
            rect
        };
        track_place(layout, space, key, moved);
    }
    // Its close window button, without a title bar, closes it as the title bar's would.
    if space.floating.close_asked.remove(dock) {
        open = false;
    }
    if !open {
        let policy = ask_close(space, viewer, key, dock);
        if policy != ClosePolicy::Keep {
            close_with(layout, space, viewer, key, policy);
        }
    }
}

/// Tears the float `key` over the main window out to a native window, when its window, at
/// `rect` and moved by the drags of `movers` (its title bar, its move handle), is let go with
/// the pointer outside every region of the work area and the work area says
/// [`Outside::Native`]: the native window opens where it was let go, the pointer where it held
/// the float. Returns whether it was torn out.
fn tear_out<Tab>(
    ctx: &Context,
    layout: &mut Layout,
    space: &mut DockSpace<Tab>,
    key: &str,
    movers: [Id; 2],
    rect: Rect,
) -> bool {
    let responses = movers.map(|id| ctx.read_response(id));
    let moving = responses.iter().flatten().any(egui::Response::dragged);
    let released = responses.iter().flatten().any(egui::Response::drag_stopped);
    let pointer = ctx.input(|i| i.pointer.latest_pos());
    if !moving && !released {
        space.floating.grabs.remove(key);
        return false;
    }
    let Some(pointer) = pointer else {
        return false;
    };
    // While over the window, where the pointer holds it.
    if rect.contains(pointer) {
        space
            .floating
            .grabs
            .insert(key.to_owned(), pointer - rect.min);
    }
    if !released {
        return false;
    }
    let grab = space.floating.grabs.remove(key);
    if space.floating.work_area.outside_rule() != Outside::Native
        || space.floating.release_host(space.docks(), pointer) != Some(FloatHost::Native)
    {
        return false;
    }
    let Some(float) = layout.float_mut(key) else {
        return false;
    };
    let grab = grab.unwrap_or_else(|| (pointer - rect.min).clamp(Vec2::ZERO, rect.size()));
    let at = Rect::from_min_size(pointer - grab, rect.size());
    // On the desktop, when the system tells where the main window is.
    let ppp = ctx.pixels_per_point();
    let physical = ctx.input(|i| i.viewport().inner_rect).map(|inner| {
        Rect::from_min_size(
            ((inner.min.to_vec2() + at.min.to_vec2()) * ppp).to_pos2(),
            at.size() * ppp,
        )
    });
    let screens = space.floating.screens.clone().unwrap_or_default();
    let place = physical
        .and_then(|rect| Place::from_physical(rect, &screens))
        .unwrap_or_else(|| Place::new(at));
    float.host = FloatHost::Native;
    float.place = place.clone();
    space.floating.chrome.remove(key);
    space.floating.collapsed.remove(key);
    space.floating.moving.remove(key);
    space.floating.placed.insert(key.to_owned(), place.rect.min);
    #[cfg(feature = "bus")]
    space.emit_change(float_change(crate::bus::Change::FloatMoved, key));
    space.floating.events.push(FloatEvent::Moved {
        key: key.to_owned(),
        place,
    });
    ctx.request_repaint();
    true
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
    gestures: &mut Gestures<'_, Tab>,
) {
    let Some(float) = layout.float(key).cloned() else {
        return;
    };
    let id = host.viewport_id(key);
    // Docking while moving needs to know where the window is; else the handle only moves it.
    let docks_on_move =
        gestures.float_move == FloatMove::Dock && host.outer_rect(ctx, id).is_some();
    let handle = move_handle_id(space.id(), dock);
    let mut moved_by_handle = None;
    space.floating.viewports.insert(key.to_owned(), id);
    // The application changed the float's place: the window is told to go there, and the
    // builder kept is made again from the place.
    let app_moved = moved_by_app(space, &float);
    let mut move_to = None;
    if app_moved {
        space.floating.builders.remove(key);
        move_to = first_builder(ViewportBuilder::default(), &float.place, screens).position;
    }
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
        if let Some(at) = move_to.take() {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::OuterPosition(at));
        }
        egui::Frame::central_panel(ui.style()).show(ui, |ui| {
            let rect = ui.available_rect_before_wrap();
            gestures
                .area(
                    &float,
                    DockArea::dock(space, dock)
                        .id(area_id)
                        .in_native_float(float.z),
                    style,
                )
                .show_inside(ui, viewer);
            gestures.decorate(ui, space, area_id, key, dock, true, rect);
        });
        if let Some(response) = ui.ctx().read_response(handle) {
            if docks_on_move {
                moved_by_handle = handle_move(ui.ctx(), space, key, &response);
            } else if response.drag_started() {
                // The system moves the window, as by its title bar.
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }
        }
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
    if let Some((desktop, released)) = moved_by_handle {
        // The pointer in the main window's points, where the system tells where it is.
        let ppp = ctx.pixels_per_point();
        let pointer = desktop.and_then(|desktop| {
            let origin = ctx.input(|i| i.viewport().inner_rect)?.min;
            Some((desktop.to_vec2() / ppp - origin.to_vec2()).to_pos2())
        });
        gestures::track(
            ctx, layout, space, viewer, style, key, dock, pointer, released,
        );
        if layout.float(key).is_none() {
            // Docked.
            return;
        }
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
    if app_moved {
        // The system tells the new position from the next frame: the place stays the
        // application's until then.
        space
            .floating
            .placed
            .insert(key.to_owned(), float.place.rect.min);
    } else {
        track_place(layout, space, key, place);
    }
}

/// Whether the application changed `float`'s place since its window was last seen.
fn moved_by_app<Tab>(space: &DockSpace<Tab>, float: &Float) -> bool {
    space
        .floating
        .placed
        .get(&float.key)
        .is_some_and(|at| *at != float.place.rect.min)
}

/// The move handle of the native float `key` moving its window itself ([`FloatMove::Dock`],
/// window positions known): the window follows the pointer. Returns, while the handle is
/// dragged or on its release, the pointer on the desktop (physical pixels) and whether it was
/// released.
fn handle_move<Tab>(
    ctx: &Context,
    space: &mut DockSpace<Tab>,
    key: &str,
    response: &egui::Response,
) -> Option<(Option<egui::Pos2>, bool)> {
    let released = response.drag_stopped();
    if !response.dragged() && !released {
        return None;
    }
    let ppp = ctx.pixels_per_point();
    let (inner, outer, pointer) = ctx.input(|i| {
        let info = i.viewport();
        (info.inner_rect, info.outer_rect, i.pointer.latest_pos())
    });
    let desktop = inner
        .zip(pointer)
        .map(|(inner, pointer)| ((inner.min.to_vec2() + pointer.to_vec2()) * ppp).to_pos2());
    let moving = space
        .floating
        .window_move
        .get_or_insert_with(|| WindowMove::new(key));
    if moving.key != key {
        *moving = WindowMove::new(key);
    }
    if moving.grab.is_none()
        && let (Some(desktop), Some(outer)) = (desktop, outer)
    {
        moving.grab = Some(desktop - (outer.min.to_vec2() * ppp).to_pos2());
    }
    // Only when the pointer moved: the system tells the pointer again, in the window's new
    // points, once the window has moved under it.
    let pointer_moved = ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
    if let (Some(desktop), Some(grab)) = (desktop, moving.grab)
        && !released
        && pointer_moved
    {
        let corner = ((desktop - grab).to_vec2() / ppp).to_pos2();
        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(corner));
    }
    moving.desktop = desktop.or(moving.desktop);
    Some((moving.desktop, released))
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
        let near = near.map(|(before, after)| crate::dock_space::Near {
            before,
            after,
            index: None,
        });
        if crate::dock_space::put_tab(space, &mut |tab| viewer.id(tab), &target, tab, near).is_err()
        {
            continue;
        }
        moved.push((id, target));
        #[cfg(feature = "bus")]
        keys.extend(key);
    }
    (moved, keys)
}
