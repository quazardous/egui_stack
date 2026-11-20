use egui::{Id, Rect, Ui, Vec2, WidgetText};

use crate::floats::{ClosePolicy, FloatClose};
use crate::{NodePath, TabDestination, TabPath, TabStyle};

/// Defines how a tab should behave and be rendered inside a [`Tree`](crate::Tree).
pub trait TabViewer {
    /// The type of tab in which you can store state to be drawn in your tabs.
    type Tab;

    /// Unique ID for this tab.
    ///
    /// Assuming `Tab` implements `Hash`, you can simply do `Id::new(tab)`.
    /// If that is not possible, but tab names are guaranteed to be unique,
    /// one could also implement this as `Id::new(self.title(tab).text())`.
    fn id(&mut self, tab: &mut Self::Tab) -> Id;

    /// The title to be displayed in the tab bar.
    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText;

    /// Actual tab content.
    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab);

    /// Content inside the context menu shown when the tab is right-clicked.
    ///
    /// `_path` is where the tab is: its [`Surface`](crate::Surface), [`Node`](crate::Node) and
    /// index, so a menu item can name it to the tab actions of [`DockState`](crate::DockState):
    /// [`close_tab`](crate::DockState::close_tab), [`eject_tab`](crate::DockState::eject_tab),
    /// [`focus_tab`](crate::DockState::focus_tab) and [`move_tab`](crate::DockState::move_tab).
    fn context_menu(&mut self, _ui: &mut Ui, _tab: &mut Self::Tab, _path: TabPath) {}

    /// Content of a tab group's menu, shown when the empty part of its tab bar is right-clicked
    /// ([`DockArea::group_context_menus`](crate::DockArea::group_context_menus)), above the
    /// built-in items. `_path` is the group's leaf. By default it adds nothing.
    fn group_context_menu(&mut self, _ui: &mut Ui, _path: NodePath) {}

    /// The application's own buttons on the tab bar of the group at `_path`, by their
    /// accessible names, in order: each gets a slot right of the bar's built-in buttons (the
    /// add button, the group grip), before its close all button, and is drawn and answered by
    /// [`on_group_bar_button`](Self::on_group_bar_button). By default none.
    fn group_bar_buttons(&mut self, _path: NodePath) -> Vec<String> {
        Vec::new()
    }

    /// Draws and answers the application's button `_index` of
    /// [`group_bar_buttons`](Self::group_bar_buttons) on the tab bar of the group at `_path`.
    /// The slot is already a button: it senses clicks, is a keyboard stop, carries its
    /// accessible name and is lit while hovered or focused; `_ui` covers it, to paint the icon,
    /// and `_response` tells its clicks (a tooltip: `_response.clone().on_hover_text(…)`).
    /// [`DockSpace::float_out_group`](crate::DockSpace::float_out_group) or
    /// [`DockState::move_node`](crate::DockState::move_node), for example, act on the group.
    fn on_group_bar_button(
        &mut self,
        _ui: &mut Ui,
        _path: NodePath,
        _index: usize,
        _response: &egui::Response,
    ) {
    }

    /// Called after each tab button is shown, so you can add a tooltip, check for clicks, etc.
    fn on_tab_button(&mut self, _tab: &mut Self::Tab, _response: &egui::Response) {}

    /// Called after the move handle of a float's window is shown
    /// ([`GroupHandles::MOVE_WINDOW`](crate::GroupHandles::MOVE_WINDOW)), with the name of the
    /// dock the float shows (its [`Content::Dock`](crate::layout::Content::Dock); a dock float
    /// is usually keyed by its dock's name).
    ///
    /// The handle moves the window by the pointer and is no keyboard stop: Tab passes it by.
    /// An application that wants it in the keyboard order makes it focusable here and moves
    /// the window itself, through the float's [`Place`](crate::layout::Place):
    ///
    /// ```rust
    /// # fn arrows(response: &egui::Response) -> egui::Vec2 {
    /// // Tab stops on the handle from now on.
    /// response
    ///     .ctx
    ///     .memory_mut(|m| m.interested_in_focus(response.id, response.layer_id));
    /// if !response.has_focus() {
    ///     return egui::Vec2::ZERO;
    /// }
    /// // The arrows are the handle's while it has the focus.
    /// response.ctx.memory_mut(|m| {
    ///     m.set_focus_lock_filter(
    ///         response.id,
    ///         egui::EventFilter {
    ///             horizontal_arrows: true,
    ///             vertical_arrows: true,
    ///             ..Default::default()
    ///         },
    ///     )
    /// });
    /// response.ctx.input(|i| {
    ///     let x = i.num_presses(egui::Key::ArrowRight) as f32
    ///         - i.num_presses(egui::Key::ArrowLeft) as f32;
    ///     let y = i.num_presses(egui::Key::ArrowDown) as f32
    ///         - i.num_presses(egui::Key::ArrowUp) as f32;
    ///     egui::vec2(x, y) * 16.0
    /// })
    /// // Then, after the frame: `layout.float_mut(key).place.rect` translated by it.
    /// # }
    /// ```
    fn on_window_handle(&mut self, _dock: &str, _response: &egui::Response) {}

    /// This is called when the `_tab` gets closed by the user.
    ///
    /// Returns an `OnCloseResponse` which determines what happens to the tab after this function gets called.
    fn on_close(&mut self, _tab: &mut Self::Tab) -> OnCloseResponse {
        OnCloseResponse::Close
    }

    /// Returns `true` if the user of your app should be able to close a given `_tab`.
    ///
    /// By default, `true` is always returned.
    fn is_closeable(&self, _tab: &Self::Tab) -> bool {
        true
    }

    /// Returns `true` if the user of your app should be able to close a given `_tab`.
    ///
    /// By default, `true` is always returned.
    #[deprecated = "Use the `TabViewer::is_closeable` function instead."]
    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        true
    }

    /// This is called every frame after [`ui`](Self::ui) is called, if the `_tab` is active.
    ///
    /// Returns `true` if the tab should be forced to close, `false` otherwise.
    ///
    /// In the event this function returns true the tab will be removed without calling `on_close`.
    fn force_close(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }

    /// This is called when the add button is pressed.
    ///
    /// `_path` specifies which [`Surface`](crate::Surface) and on which
    /// [`Node`](crate::Node) this particular add button was pressed.
    fn on_add(&mut self, _path: NodePath) {}

    /// Called when the rectangle of the tab content changes.
    ///
    /// This can happen when the window is resized, panels are docked or undocked,
    /// or when the layout of the dock area is changed in any way that affects
    /// the available space for the tab content.
    ///
    /// This is useful for tabs that need to adjust their content based on the
    /// available space.
    fn on_rect_changed(&mut self, _tab: &mut Self::Tab) {}

    /// Content of the popup under the add button. Useful for selecting what type of tab to add.
    ///
    /// This requires that [`DockArea::show_add_buttons`](crate::DockArea::show_add_buttons) and
    /// [`DockArea::show_add_popup`](crate::DockArea::show_add_popup) are set to `true`.
    fn add_popup(&mut self, _ui: &mut Ui, _path: NodePath) {}

    /// Sets custom style for given tab.
    fn tab_style_override(&self, _tab: &Self::Tab, _global_style: &TabStyle) -> Option<TabStyle> {
        None
    }

    /// Specifies a tab's ability to be shown in a window.
    ///
    /// Returns `false` if this tab should never be turned into a window.
    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        true
    }

    /// Whether `tab` may be dropped at `destination`.
    ///
    /// Asked every frame while `tab` is dragged over a destination the dock would otherwise accept
    /// (windows refused by [`allowed_in_windows`](Self::allowed_in_windows) are never offered).
    /// A [`DropVerdict::Refuse`] is shown on the drop overlay, with its reason if it has one, and
    /// releasing the tab there leaves the layout unchanged.
    ///
    /// By default every destination is allowed.
    fn drop_allowed(&self, _tab: &Self::Tab, _destination: &TabDestination) -> DropVerdict {
        DropVerdict::Allow
    }

    /// Whether `tab` may be dropped at `drop.destination`, knowing the docks of a
    /// [`DockSpace`](crate::DockSpace) it would leave and enter.
    ///
    /// Asked as [`drop_allowed`](Self::drop_allowed) is, by every [`DockArea`](crate::DockArea),
    /// inside one dock and between the docks of a space alike; outside a space, both docks are
    /// `None`. By default it returns what [`drop_allowed`](Self::drop_allowed) does, so a viewer
    /// overrides one or the other.
    fn dock_drop_allowed(&self, tab: &Self::Tab, drop: &DockDrop<'_>) -> DropVerdict {
        self.drop_allowed(tab, drop.destination)
    }

    /// The title shown for the dock named `dock` of a [`DockSpace`](crate::DockSpace), where
    /// the dock names a place the user picks: the docks listed by the Move group menu
    /// ([`GroupHandles::MENU`](crate::GroupHandles::MENU)), whose items, and their accessible
    /// names, read this title. A refused dock's item is greyed with the reason
    /// [`dock_drop_allowed`](Self::dock_drop_allowed) gives, which the viewer words itself, so
    /// it can name the dock by the same title.
    ///
    /// By default `None`: the dock is shown by its name, as given to
    /// [`DockSpace::add_dock`](crate::DockSpace::add_dock). A float's dock the space named itself
    /// (`float-N`, when tabs are moved out to a new float, or `window-N`, from
    /// [`DockSpace::windows_to_floats`](crate::DockSpace::windows_to_floats)) is shown instead by
    /// its window's title: the active tab's [`title`](Self::title) of its focused, else first,
    /// group. It is recognised by its name, so `float-<digits>` and `window-<digits>` are
    /// reserved for those docks: a dock the application names so is shown the same way.
    fn dock_title(&mut self, _dock: &str) -> Option<WidgetText> {
        None
    }

    /// What becomes of the tabs of a float the user closes (its close button, or its native
    /// window's): `close` names the float, its dock and its tabs. By default
    /// [`ClosePolicy::ReturnToOrigin`]: each tab goes back where it came from.
    /// [`ClosePolicy::Keep`] cancels the close.
    ///
    /// With the `bus` feature and a bus given to the space, the answer is the start value of the
    /// `stack.window.close` request, whose listeners may change it.
    fn close_policy(&mut self, _close: &FloatClose) -> ClosePolicy {
        ClosePolicy::default()
    }

    /// Whether the tab body will be cleared with the color specified in
    /// [`TabBarStyle::bg_fill`](crate::TabBarStyle::bg_fill).
    fn clear_background(&self, _tab: &Self::Tab) -> bool {
        true
    }

    /// Returns `true` if the horizontal and vertical scroll bars will be shown for `tab`.
    ///
    /// By default, both scroll bars are shown.
    fn scroll_bars(&self, _tab: &Self::Tab) -> [bool; 2] {
        [true, true]
    }

    /// The size of the ghost under the pointer while `tab` is dragged, or while its group is
    /// dragged with `tab` as its active tab: its tab bar on top, its body below. `default` is
    /// the size egui_stack picks, its leaf's size kept within limits (160 × 80 to 320 × 200
    /// points); whatever is returned is kept within the same limits.
    fn ghost_size(&mut self, _tab: &Self::Tab, default: Vec2) -> Vec2 {
        default
    }

    /// Paints the body of the ghost under the pointer while `tab` is dragged, or while its
    /// group is dragged with `tab` as its active tab: a live preview of its content, for
    /// instance. `ui` covers `rect`, the ghost's body below its tab bar, on the ghost's layer,
    /// clipped to it and as translucent as the rest of the ghost; the frame, the shadow and
    /// the tab bar around it are egui_stack's ([`DockPainter`](crate::DockPainter)). Returns
    /// whether it painted the body: by default it paints nothing and returns `false`, and the
    /// painter shows its plain hint of the body instead.
    fn ghost_body(&mut self, _ui: &mut Ui, _tab: &Self::Tab, _rect: Rect) -> bool {
        false
    }
}

/// Whether a dragged tab may be dropped at a destination, from [`TabViewer::drop_allowed`].
#[derive(Clone, Debug, Default)]
pub enum DropVerdict {
    /// The tab may be dropped there.
    #[default]
    Allow,
    /// The tab may not be dropped there. The reason, if any, is shown next to the pointer.
    Refuse(Option<WidgetText>),
}

/// Where a dragged tab would be dropped, for [`TabViewer::dock_drop_allowed`].
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct DockDrop<'a> {
    /// Where the tab would go.
    pub destination: &'a TabDestination,
    /// The name of the dock the tab leaves, `None` for a floating window or outside a
    /// [`DockSpace`](crate::DockSpace).
    pub from: Option<&'a str>,
    /// The name of the dock the tab enters, `None` for a floating window (a new one included) or
    /// outside a [`DockSpace`](crate::DockSpace).
    pub to: Option<&'a str>,
}

/// Determines what happens to a tab when a user attempts to close it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OnCloseResponse {
    /// Closes the tab.
    Close,
    /// Focuses on the tab.
    Focus,
    /// Ignores the close request.
    Ignore,
}
