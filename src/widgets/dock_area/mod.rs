/// Due to there being a lot of code to show a dock in a ui every complementing
/// method to ``show`` and ``show_inside`` is put in ``show_extra``.
/// Otherwise ``mod.rs`` would be humongous.
mod show;

// Various components of the `DockArea` which is used when rendering
mod allowed_splits;
mod drag_and_drop;
mod labels;
mod state;
mod tab_removal;

pub use allowed_splits::AllowedSplits;
use egui::{Context, Id, Modifiers, emath::*};
pub use labels::Labels;
use tab_removal::TabRemoval;

use crate::{NodePath, Style, TabIndex, TabPath, dock_state::DockState};

/// Displays a [`DockState`] in `egui`.
pub struct DockArea<'tree, Tab> {
    id: Id,
    dock_state: &'tree mut DockState<Tab>,
    style: Option<Style>,
    labels: Labels,
    show_add_popup: bool,
    show_add_buttons: bool,
    show_close_buttons: bool,
    tab_context_menus: bool,
    show_builtin_tab_menu: bool,
    draggable_tabs: bool,
    tab_focus: TabFocus,
    hidable_tab_bars: bool,
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

    to_remove: Vec<TabRemoval>,
    to_detach: Vec<TabPath>,
    new_focused: Option<NodePath>,
    tab_hover_rect: Option<(Rect, TabIndex)>,
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
}

/// How the keyboard focus reaches the tabs of a tab bar, set with [`DockArea::tab_focus`].
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
            labels: Labels::ENGLISH,
            show_add_popup: false,
            show_add_buttons: false,
            show_close_buttons: true,
            tab_context_menus: true,
            show_builtin_tab_menu: true,
            draggable_tabs: true,
            tab_focus: TabFocus::Each,
            hidable_tab_bars: false,
            show_tab_name_on_hover: false,
            allowed_splits: AllowedSplits::default(),
            to_remove: Vec::new(),
            to_detach: Vec::new(),
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
