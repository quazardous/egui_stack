/// Due to there being a lot of code to show a dock in a ui every complementing
/// method to ``show`` and ``show_inside`` is put in ``show_extra``.
/// Otherwise ``mod.rs`` would be humongous.
mod show;

// Various components of the `DockArea` which is used when rendering
mod allowed_splits;
mod drag_and_drop;
mod drop_zone;
mod group_handles;
mod labels;
pub(crate) mod session;
mod state;
mod tab_removal;

pub use allowed_splits::AllowedSplits;
pub use drop_zone::DropZone;
pub use group_handles::GroupHandles;
use std::panic::AssertUnwindSafe;

use egui::{Context, Id, Modifiers, Response, Ui, emath::*};
pub use labels::Labels;
pub use session::{DragPayload, SpaceDrag};
use tab_removal::TabRemoval;

use crate::{
    DockPainter, DockSpace, NodePath, Style, StylePainter, TabIndex, TabPath,
    dock_space::{Dock, NoDrop, TabOrigin},
    dock_state::DockState,
    floats::Floating,
};

/// Displays a [`DockState`] in `egui`.
pub struct DockArea<'tree, Tab> {
    id: Id,
    dock_state: &'tree mut DockState<Tab>,
    style: Option<Style>,
    // Asserted unwind safe so that `DockArea` keeps its auto traits: the painter only paints.
    painter: AssertUnwindSafe<&'tree (dyn DockPainter + Sync)>,
    labels: Labels,
    show_add_popup: bool,
    show_add_buttons: bool,
    show_close_buttons: bool,
    tab_context_menus: bool,
    show_builtin_tab_menu: bool,
    draggable_tabs: bool,
    tab_focus: TabFocus,
    hidable_tab_bars: bool,
    hidden_tab_bar_drags_group: bool,
    group_handles: GroupHandles,
    group_drag_modifiers: Modifiers,
    show_tab_name_on_hover: bool,
    show_window_close_buttons: bool,
    show_window_collapse_buttons: bool,
    show_leaf_close_all_buttons: bool,
    show_leaf_collapse_buttons: bool,
    show_secondary_button_hint: bool,
    secondary_button_modifiers: Modifiers,
    secondary_button_on_modifier: bool,
    secondary_button_context_menu: bool,
    allowed_splits: AllowedSplits,
    window_bounds: Option<Rect>,

    /// Set when the area shows one dock of a [`DockSpace`](crate::DockSpace).
    space: Option<SpaceDock<'tree, Tab>>,

    to_remove: Vec<TabRemoval>,
    to_detach: Vec<TabPath>,
    /// Groups moved by their Move group menu, applied once the area is drawn.
    to_move_group: Vec<(NodePath, crate::TabDestination)>,
    /// Tabs and groups moved out to a new float, applied once the area is drawn.
    to_float: Vec<(drag_and_drop::TreeComponent, Rect)>,
    new_focused: Option<NodePath>,
    tab_hover_rect: Option<(Rect, TabIndex)>,
}

/// The dock of a [`DockSpace`](crate::DockSpace) a [`DockArea`] shows, and the space's docks.
struct SpaceDock<'a, Tab> {
    docks: &'a mut Vec<Dock>,
    /// Where no tab is dropped, this frame.
    no_drop: &'a mut NoDrop,
    /// The floats' side of the space.
    floating: &'a mut Floating,
    /// Where tabs moved out to a float came from.
    origins: &'a mut Vec<TabOrigin>,
    /// The application's bus, if it gave one.
    #[cfg(feature = "bus")]
    bus: Option<&'a crate::bus::SpaceBus<Tab>>,
    #[cfg(not(feature = "bus"))]
    _tab: std::marker::PhantomData<fn(&Tab)>,
    /// The space's id, which keys its floats' move handles.
    space_id: Id,
    /// The dock shown, as an index in `docks`; `None` if the space has no dock of that name.
    dock: Option<usize>,
    /// Whether this area also shows the floating windows.
    shows_windows: bool,
    /// Whether this area draws a dock in a native float's viewport: the space's no-drop zones,
    /// strips and bands are the main window's, and it leaves them alone.
    native: bool,
    /// The viewport the area is drawn in, this frame.
    viewport: Option<egui::ViewportId>,
    /// The rank of the area's viewport for a drag across viewports: 0 in the main window, a
    /// native float's z order plus one in its window.
    rank: u32,
    /// Whether the area draws a float over the main window that has no title bar: its top tab
    /// bar has a close window button beside the move handle.
    close_button: bool,
    /// The folded zone whose strip, or the collapsed zone whose restore band, the drag hovers
    /// this frame, with the dock it takes the tab for: the zone a drop there names.
    drop_zone: Option<(String, crate::layout::ZoneName)>,
}

/// A tab being dragged in a [`DockArea`], from [`DockArea::dragged_tab`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DraggedTab {
    /// Where the dragged tab is in the [`DockState`].
    pub path: TabPath,
    /// The pointer's position.
    pub pointer: Pos2,
}

/// A tab group being dragged in a [`DockArea`], from [`DockArea::dragged_node`]: a leaf with
/// all its tabs, dragged by one of its [`GroupHandles`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DraggedNode {
    /// Where the dragged leaf is in the [`DockState`].
    pub path: NodePath,
    /// The pointer's position.
    pub pointer: Pos2,
}

// Not generic over the tab type, so that `DockArea::dragged_tab(ctx, id)` needs no annotation.
impl DockArea<'_, ()> {
    /// The id of a [`DockArea`] whose [`id`](Self::id) was not set.
    pub fn default_id() -> Id {
        Id::unique("egui_stack::DockArea")
    }

    /// The tab being dragged in the [`DockArea`] with this `id`, as of its last
    /// [`show_inside`](DockArea::show_inside).
    ///
    /// A tab counts as dragged once it has moved far enough to leave its tab bar, until it is
    /// released. Use [`default_id`](Self::default_id) if the [`DockArea`]'s id was not set.
    pub fn dragged_tab(ctx: &Context, id: Id) -> Option<DraggedTab> {
        ctx.data(|d| d.get_temp::<state::State>(id))
            .and_then(|state| state.dragged)
    }

    /// The tab bars of the [`DockArea`] with this `id` drawn in the current pass (the last
    /// one, once the pass ended), each with its leaf, in the area's viewport's points: the
    /// whole bar, margins included. A hidden tab bar is not listed. The docks of a
    /// [`DockSpace`] drawn in one viewport share their id, so their bars are listed together;
    /// a leaf's [`NodePath::surface`] tells its dock ([`DockSpace::dock_surface`]).
    ///
    /// An application draws something of its own over a bar with it, such as a button at its
    /// right end. Use [`default_id`](Self::default_id) if the [`DockArea`]'s id was not set.
    pub fn tab_bars(ctx: &Context, id: Id) -> Vec<(NodePath, Rect)> {
        ctx.data(|d| d.get_temp::<state::State>(id))
            .map(|state| state.tab_bars)
            .unwrap_or_default()
    }

    /// The tab group being dragged in the [`DockArea`] with this `id`, as of its last
    /// [`show_inside`](DockArea::show_inside): a leaf dragged whole by one of its
    /// [`GroupHandles`], all its tabs moving together and its active tab kept.
    ///
    /// It counts as dragged once it has moved far enough, until it is released. Use
    /// [`default_id`](Self::default_id) if the [`DockArea`]'s id was not set.
    pub fn dragged_node(ctx: &Context, id: Id) -> Option<DraggedNode> {
        ctx.data(|d| d.get_temp::<state::State>(id))
            .and_then(|state| state.dragged_node)
    }

    /// Makes `response` a handle that drags the whole tab group showing it, as the empty part of
    /// a tab bar does: call it from [`TabViewer::ui`](crate::TabViewer::ui) with the response of
    /// a widget of the tab's body that senses drags, such as an editor's own header. It lets an
    /// editor whose tab bar is hidden be moved by a handle the app draws where it wants.
    ///
    /// Dragged past [`OverlayFeel::drag_activation_distance`](crate::OverlayFeel::drag_activation_distance),
    /// the group moves through the same drop zones as a tab, and
    /// [`dragged_node`](Self::dragged_node) reports it. It does nothing outside a tab's body, or
    /// when the [`DockArea`]'s tabs are not [`draggable`](DockArea::draggable_tabs). Returns
    /// whether `response` is being dragged as a handle.
    pub fn drag_group_by(ui: &Ui, response: &Response) -> bool {
        let Some(body) = ui.data(|d| d.get_temp::<Option<BodyLeaf>>(BodyLeaf::id()).flatten())
        else {
            return false;
        };
        if !body.draggable {
            return false;
        }
        let dragged = response.dragged() && ui.input(|i| i.pointer.is_decidedly_dragging());
        if dragged {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
            ui.data_mut(|d| d.insert_temp(BodyLeaf::request_id(body.dock_area), body.path));
        } else if response.hovered() && ui.ctx().dragged_id().is_none() {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
        }
        dragged
    }
}

/// The leaf whose tab body is being shown, so that [`DockArea::drag_group_by`] knows which
/// group a handle drags. Kept in the context's temporary data while
/// [`TabViewer::ui`](crate::TabViewer::ui) runs, the outer one put back after (a dock shown
/// inside a tab).
#[derive(Clone, Copy)]
struct BodyLeaf {
    dock_area: Id,
    path: NodePath,
    draggable: bool,
}

impl BodyLeaf {
    fn id() -> Id {
        Id::unique("egui_stack::BodyLeaf")
    }

    /// Where a handle asks the [`DockArea`] `dock_area` to drag a group this frame.
    fn request_id(dock_area: Id) -> Id {
        dock_area.with("group_drag_request")
    }
}

/// How the keyboard focus reaches the tabs of a tab bar, set with [`DockArea::tab_focus`], or
/// the tools of a [`ToolRow`](crate::toolbar::ToolRow), set with
/// [`ToolRow::tab_focus`](crate::toolbar::ToolRow::tab_focus) (where [`Roving`](Self::Roving)
/// is the default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TabFocus {
    /// Every tab, and every tab's close button, is a Tab stop.
    #[default]
    Each,
    /// One Tab stop per tab bar, the active tab, as in WAI-ARIA's tab list: once it has the
    /// focus, Left and Right select and focus the previous and next tab (wrapping around), Home
    /// and End the first and last. Close buttons are not Tab stops; they stay clickable.
    Roving,
}

// Builder
impl<'tree, Tab> DockArea<'tree, Tab> {
    /// Creates a new [`DockArea`] from the provided [`DockState`].
    #[inline(always)]
    pub fn new(tree: &'tree mut DockState<Tab>) -> DockArea<'tree, Tab> {
        Self {
            id: DockArea::default_id(),
            dock_state: tree,
            style: None,
            painter: AssertUnwindSafe(&StylePainter),
            labels: Labels::ENGLISH,
            show_add_popup: false,
            show_add_buttons: false,
            show_close_buttons: true,
            tab_context_menus: true,
            show_builtin_tab_menu: true,
            draggable_tabs: true,
            tab_focus: TabFocus::Each,
            hidable_tab_bars: false,
            hidden_tab_bar_drags_group: false,
            group_handles: GroupHandles::ALL,
            group_drag_modifiers: Modifiers::SHIFT,
            show_tab_name_on_hover: false,
            allowed_splits: AllowedSplits::default(),
            space: None,
            to_remove: Vec::new(),
            to_detach: Vec::new(),
            to_move_group: Vec::new(),
            to_float: Vec::new(),
            new_focused: None,
            tab_hover_rect: None,
            window_bounds: None,
            show_window_close_buttons: true,
            show_window_collapse_buttons: true,
            show_leaf_close_all_buttons: true,
            show_leaf_collapse_buttons: true,
            show_secondary_button_hint: true,
            secondary_button_modifiers: Modifiers::SHIFT,
            secondary_button_on_modifier: true,
            secondary_button_context_menu: true,
        }
    }

    /// Creates a [`DockArea`] showing the dock named `dock` of `space`, drawn in the [`Ui`] given
    /// to [`show_inside`](Self::show_inside); if the dock is the space's
    /// [windows dock](crate::DockSpace::windows_dock), the floating windows too.
    ///
    /// Each dock of a space is shown by its own call, in any order and anywhere in the frame;
    /// a tab is dragged from any of them to any other as inside one dock. The calls share the
    /// space's [`id`](crate::DockSpace::id): set it on the space, not with [`id`](Self::id). A
    /// dock the space does not have shows nothing.
    ///
    /// [`Ui`]: egui::Ui
    pub fn dock(space: &'tree mut DockSpace<Tab>, dock: &str) -> DockArea<'tree, Tab> {
        let parts = space.split_for_area(dock);
        let mut area = Self::new(parts.state).id(parts.id);
        area.space = Some(SpaceDock {
            docks: parts.docks,
            no_drop: parts.no_drop,
            floating: parts.floating,
            origins: parts.origins,
            #[cfg(feature = "bus")]
            bus: parts.bus,
            #[cfg(not(feature = "bus"))]
            _tab: std::marker::PhantomData,
            space_id: parts.id,
            dock: parts.dock,
            shows_windows: parts.shows_windows,
            native: false,
            viewport: None,
            rank: 0,
            close_button: false,
            drop_zone: None,
        });
        area
    }

    /// Marks the area as drawing a float over the main window that has no title bar of its
    /// own: its top tab bar gets a close window button beside the move handle, which asks the
    /// [`FloatsArea`](crate::FloatsArea) to close the float.
    pub(crate) fn float_close_button(mut self) -> Self {
        if let Some(space) = &mut self.space {
            space.close_button = true;
        }
        self
    }

    /// Marks the area as drawing a dock in a native float's viewport, the float's z order
    /// `z`: it keeps its own drag state, takes part in the space's drag across viewports, and
    /// leaves the main window's no-drop zones alone.
    pub(crate) fn in_native_float(mut self, z: u32) -> Self {
        if let Some(space) = &mut self.space {
            space.native = true;
            space.shows_windows = false;
            space.rank = z.saturating_add(1);
        }
        self
    }

    /// Sets the [`DockArea`] ID. Useful if you have more than one [`DockArea`].
    #[inline(always)]
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    /// Sets the look and feel of the [`DockArea`].
    #[inline(always)]
    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets what paints the tabs, tab bars, separators and drop overlay. By default it's
    /// [`StylePainter`], which paints them from the [`Style`].
    ///
    /// The painter is `Sync`, as the rest of a [`DockArea`] can be: one that records what it
    /// paints keeps it behind a `Mutex`.
    ///
    /// ```
    /// # use egui_stack::{DockArea, DockPainter, DockState, SeparatorPaint};
    /// /// Separators as thin lines, everything else as the style says.
    /// struct ThinSeparators;
    ///
    /// impl DockPainter for ThinSeparators {
    ///     fn separator(&self, painter: &egui::Painter, separator: &SeparatorPaint<'_>) {
    ///         let color = separator.style.separator.color_hovered;
    ///         painter.rect_filled(separator.rect.shrink(1.0), 0.0, color);
    ///     }
    /// }
    ///
    /// # egui::__run_test_ui(|ui| {
    /// # struct Viewer;
    /// # impl egui_stack::TabViewer for Viewer {
    /// #     type Tab = String;
    /// #     fn id(&mut self, tab: &mut String) -> egui::Id { egui::Id::new(tab.as_str()) }
    /// #     fn title(&mut self, tab: &mut String) -> egui::WidgetText { tab.as_str().into() }
    /// #     fn ui(&mut self, ui: &mut egui::Ui, tab: &mut String) { ui.label(tab.as_str()); }
    /// # }
    /// let mut dock_state = DockState::new(vec!["Tab".to_owned()]);
    /// DockArea::new(&mut dock_state)
    ///     .painter(&ThinSeparators)
    ///     .show_inside(ui, &mut Viewer);
    /// # });
    /// ```
    ///
    /// A painter can draw what no style field gives, such as a border on the focused leaf
    /// alone, glowing:
    ///
    /// ```
    /// # use egui_stack::{DockArea, DockPainter, DockState, PartState, TabBodyPaint};
    /// /// A glow inside the focused leaf's body, and no border on the others.
    /// struct FocusGlow;
    ///
    /// impl DockPainter for FocusGlow {
    ///     fn tab_body(&self, painter: &egui::Painter, body: &TabBodyPaint<'_>) {
    ///         if !body.state.contains(PartState::FOCUSED) {
    ///             return;
    ///         }
    ///         let color = egui::Color32::from_rgb(0, 160, 255);
    ///         for (inset, alpha) in [(0.5, 1.0), (1.5, 0.4), (2.5, 0.15)] {
    ///             painter.rect_stroke(
    ///                 body.rect.shrink(inset),
    ///                 body.corner_radius,
    ///                 egui::Stroke::new(1.0, color.gamma_multiply(alpha)),
    ///                 egui::StrokeKind::Middle,
    ///             );
    ///         }
    ///     }
    /// }
    ///
    /// # egui::__run_test_ui(|ui| {
    /// # struct Viewer;
    /// # impl egui_stack::TabViewer for Viewer {
    /// #     type Tab = String;
    /// #     fn id(&mut self, tab: &mut String) -> egui::Id { egui::Id::new(tab.as_str()) }
    /// #     fn title(&mut self, tab: &mut String) -> egui::WidgetText { tab.as_str().into() }
    /// #     fn ui(&mut self, ui: &mut egui::Ui, tab: &mut String) { ui.label(tab.as_str()); }
    /// # }
    /// let mut dock_state = DockState::new(vec!["Tab".to_owned()]);
    /// DockArea::new(&mut dock_state)
    ///     .painter(&FocusGlow)
    ///     .show_inside(ui, &mut Viewer);
    /// # });
    /// ```
    #[inline(always)]
    pub fn painter(mut self, painter: &'tree (dyn DockPainter + Sync)) -> Self {
        self.painter = AssertUnwindSafe(painter);
        self
    }

    /// Sets the accessible names of the buttons and separators, read by screen readers through
    /// AccessKit. By default they are [`Labels::ENGLISH`].
    #[inline(always)]
    pub fn labels(mut self, labels: Labels) -> Self {
        self.labels = labels;
        self
    }

    /// Sets how the keyboard focus reaches the tabs. By default it's [`TabFocus::Each`].
    #[inline(always)]
    pub fn tab_focus(mut self, tab_focus: TabFocus) -> Self {
        self.tab_focus = tab_focus;
        self
    }

    /// Shows or hides the add button popup.
    /// By default it's `false`.
    pub fn show_add_popup(mut self, show_add_popup: bool) -> Self {
        self.show_add_popup = show_add_popup;
        self
    }

    /// Shows or hides the tab add buttons.
    /// By default it's `false`.
    pub fn show_add_buttons(mut self, show_add_buttons: bool) -> Self {
        self.show_add_buttons = show_add_buttons;
        self
    }

    /// Shows or hides the tab close buttons.
    /// By default it's `true`.
    pub fn show_close_buttons(mut self, show_close_buttons: bool) -> Self {
        self.show_close_buttons = show_close_buttons;
        self
    }

    /// Whether tabs show a context menu when right-clicked.
    /// By default it's `true`.
    pub fn tab_context_menus(mut self, tab_context_menus: bool) -> Self {
        self.tab_context_menus = tab_context_menus;
        self
    }

    /// Shows or hides the built-in items of the tab context menus: Eject, Close, and Hide or Show
    /// tab bar. Hidden, a tab's menu holds only what [`TabViewer::context_menu`] adds, for
    /// applications that offer those actions as their own commands.
    /// By default it's `true`.
    ///
    /// [`TabViewer::context_menu`]: crate::TabViewer::context_menu
    #[inline(always)]
    pub fn show_builtin_tab_menu(mut self, show_builtin_tab_menu: bool) -> Self {
        self.show_builtin_tab_menu = show_builtin_tab_menu;
        self
    }

    /// Whether tabs can be dragged between nodes and reordered on the tab bar.
    /// By default it's `true`.
    pub fn draggable_tabs(mut self, draggable_tabs: bool) -> Self {
        self.draggable_tabs = draggable_tabs;
        self
    }

    /// Whether tab bars can be hidden with the context menu.
    /// By default it's `false`.
    pub fn hidable_tab_bars(mut self, hidable_tab_bars: bool) -> Self {
        self.hidable_tab_bars = hidable_tab_bars;
        self
    }

    /// Whether the strip left at the top of a leaf whose tab bar is hidden drags the whole tab
    /// group, as the empty part of a tab bar does, rather than its active tab alone.
    /// By default it's `false`: the strip drags the active tab.
    ///
    /// The strip is [`TabBodyStyle::hidden_tab_bar_drag_height`](crate::TabBodyStyle::hidden_tab_bar_drag_height)
    /// high. [`DockArea::drag_group_by`] makes any widget of the tab's body a handle instead.
    pub fn hidden_tab_bar_drags_group(mut self, drags_group: bool) -> Self {
        self.hidden_tab_bar_drags_group = drags_group;
        self
    }

    /// Which handles move a whole tab group: the empty part of the tab bar, a grip at the end of
    /// each tab bar, a tab dragged with [`group_drag_modifiers`](Self::group_drag_modifiers)
    /// held, and "Move group" in the tab's context menu. By default all of them
    /// ([`GroupHandles::ALL`]); an application narrows the set to what it allows, and a user
    /// setting can narrow it further with [`GroupHandles::intersection`].
    ///
    /// No handle moves anything when the tabs are not [`draggable`](Self::draggable_tabs), the
    /// menu aside, which moves the group without a drag.
    #[inline(always)]
    pub fn group_handles(mut self, group_handles: GroupHandles) -> Self {
        self.group_handles = group_handles;
        self
    }

    /// The modifiers that, held when a tab starts being dragged, make the drag carry the tab's
    /// whole group ([`GroupHandles::MODIFIER_DRAG`]). By default [`Modifiers::SHIFT`].
    ///
    /// Shift is free on a tab drag: egui gives a drag no meaning of its own, and egui_stack's
    /// other modifiers are Shift on the tab bar's buttons, a click, not a drag
    /// ([`secondary_button_modifiers`](Self::secondary_button_modifiers)), and Ctrl (Cmd on
    /// macOS) for a [`Sections`](crate::sections::Sections) pull. Applications that dock with
    /// Ctrl or Alt keep Shift for the group. They are matched as
    /// [`Modifiers::matches_logically`] does; [`Modifiers::NONE`] turns this handle off, as
    /// clearing its flag does.
    #[inline(always)]
    pub fn group_drag_modifiers(mut self, modifiers: Modifiers) -> Self {
        self.group_drag_modifiers = modifiers;
        self
    }

    /// Whether tabs show their name when hovered over them.
    /// By default it's `false`.
    pub fn show_tab_name_on_hover(mut self, show_tab_name_on_hover: bool) -> Self {
        self.show_tab_name_on_hover = show_tab_name_on_hover;
        self
    }

    /// What directions can a node be split in: left-right, top-bottom, all, or none.
    /// By default it's all.
    pub fn allowed_splits(mut self, allowed_splits: AllowedSplits) -> Self {
        self.allowed_splits = allowed_splits;
        self
    }

    /// Whether tooltip hints are shown for secondary buttons on tab bars.
    /// By default it's `true`.
    pub fn show_secondary_button_hint(mut self, show_secondary_button_hint: bool) -> Self {
        self.show_secondary_button_hint = show_secondary_button_hint;
        self
    }

    /// The key combination used to activate secondary buttons on tab bars.
    /// By default it's [`Modifiers::SHIFT`].
    pub fn secondary_button_modifiers(mut self, secondary_button_modifiers: Modifiers) -> Self {
        self.secondary_button_modifiers = secondary_button_modifiers;
        self
    }

    /// Whether the secondary buttons on tab bars are activated by the modifier key.
    /// By default it's `true`.
    pub fn secondary_button_on_modifier(mut self, secondary_button_on_modifier: bool) -> Self {
        self.secondary_button_on_modifier = secondary_button_on_modifier;
        self
    }

    /// Whether the secondary buttons on tab bars are activated from a context value by right-clicking primary buttons.
    /// By default it's `true`.
    pub fn secondary_button_context_menu(mut self, secondary_button_context_menu: bool) -> Self {
        self.secondary_button_context_menu = secondary_button_context_menu;
        self
    }

    /// The bounds for any windows inside the [`DockArea`]. Defaults to the screen rect.
    /// By default it's set to [`egui::Context::content_rect`].
    #[inline(always)]
    pub fn window_bounds(mut self, bounds: Rect) -> Self {
        self.window_bounds = Some(bounds);
        self
    }

    /// Enables or disables the close button on windows.
    /// By default it's `true`.
    #[inline(always)]
    #[deprecated = "consider using `show_leaf_close_buttons` instead."]
    pub fn show_window_close_buttons(mut self, show_window_close_buttons: bool) -> Self {
        self.show_window_close_buttons = show_window_close_buttons;
        self
    }

    /// Enables or disables the collapsing header on windows.
    /// By default it's `true`.
    #[inline(always)]
    #[deprecated = "consider using `show_leaf_collapse_buttons` instead."]
    pub fn show_window_collapse_buttons(mut self, show_window_collapse_buttons: bool) -> Self {
        self.show_window_collapse_buttons = show_window_collapse_buttons;
        self
    }

    /// Enables or disables the close all tabs button on tab bars.
    /// By default it's `true`.
    #[inline(always)]
    pub fn show_leaf_close_all_buttons(mut self, show_leaf_close_all_buttons: bool) -> Self {
        self.show_leaf_close_all_buttons = show_leaf_close_all_buttons;
        self
    }

    /// Enables or disables the collapse tabs button on tab bars.
    /// By default it's `true`.
    #[inline(always)]
    pub fn show_leaf_collapse_buttons(mut self, show_leaf_collapse_buttons: bool) -> Self {
        self.show_leaf_collapse_buttons = show_leaf_collapse_buttons;
        self
    }
}

impl<Tab> std::fmt::Debug for DockArea<'_, Tab> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DockArea").finish_non_exhaustive()
    }
}
