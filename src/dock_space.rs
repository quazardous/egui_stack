//! Several named docks sharing one drag: see [`DockSpace`].

use egui::{Id, Rect, ViewportId};

use crate::floats::{FloatEvent, Floating, PendingFloat};
use crate::layout::{
    Content, Drops, Fold, Laid, LaidZone, Layout, Restore, Shown, Zone, ZoneKind, ZoneName,
};
use crate::layout::{Float, FloatHost, Place};
use crate::{AllowedSplits, DockState, NodePath, Surface, SurfaceIndex, TabPath, Tree};

/// Several named docks, each with its own tree, and floating windows, sharing one drag: a tab
/// dragged from any dock to any other gets the same drop zones, preview and veto as inside one
/// dock.
///
/// Each dock is drawn by its own [`DockArea::dock`](crate::DockArea::dock) call, inside the
/// [`Ui`](egui::Ui) the application lays out for it (a side panel, the central panel, a flyout),
/// at any point of the frame. The calls share the drag through the space's [`id`](Self::id).
/// One dock, the [windows dock](Self::set_windows_dock), also draws the floating windows.
///
/// The docks are surfaces of one [`DockState`] ([`Surface::Main`]), so a [`TabPath`]
/// or a [`TabDestination`](crate::TabDestination) names a place in any of them, and
/// [`DockState::move_tab`] moves a tab between them; [`dock_surface`](Self::dock_surface) and
/// [`dock_name`](Self::dock_name) go between a dock's name and its surface.
///
/// ```
/// # use egui_stack::{AllowedSplits, DockArea, DockRules, DockSpace, TabViewer};
/// # struct Viewer;
/// # impl TabViewer for Viewer {
/// #     type Tab = String;
/// #     fn id(&mut self, tab: &mut String) -> egui::Id { egui::Id::unique(tab.as_str()) }
/// #     fn title(&mut self, tab: &mut String) -> egui::WidgetText { tab.as_str().into() }
/// #     fn ui(&mut self, ui: &mut egui::Ui, tab: &mut String) { ui.label(tab.as_str()); }
/// # }
/// let mut space = DockSpace::new();
/// space.add_dock(
///     "left",
///     DockRules::default().splits(AllowedSplits::TopBottomOnly),
///     vec!["Files".to_owned()],
/// );
/// space.add_dock("centre", DockRules::default(), vec!["Editor".to_owned()]);
///
/// # egui::__run_test_ui(|ui| {
/// egui::Panel::left("left").show(ui, |ui| {
///     DockArea::dock(&mut space, "left").show_inside(ui, &mut Viewer);
/// });
/// egui::CentralPanel::default().show(ui, |ui| {
///     DockArea::dock(&mut space, "centre").show_inside(ui, &mut Viewer);
/// });
/// # });
/// ```
///
/// # Serialization
///
/// With the `serde` feature, a space is one value: its [`DockState`], its docks with their
/// rules, and its windows dock. Its [`id`](Self::id) is not saved.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(bound(
        deserialize = "Tab: serde::Deserialize<'de>",
        serialize = "Tab: serde::Serialize"
    ))
)]
pub struct DockSpace<Tab> {
    state: DockState<Tab>,
    docks: Vec<Dock>,
    windows_dock: Option<String>,
    /// Where tabs moved out to a float came from, for them to go back.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    origins: Vec<TabOrigin>,
    #[cfg_attr(
        feature = "serde",
        serde(skip, default = "DockSpace::<()>::default_id")
    )]
    id: Id,
    /// The rects no tab is dropped in, this frame.
    #[cfg_attr(feature = "serde", serde(skip))]
    no_drop: NoDrop,
    /// The floats' side of the space: tabs moved out waiting for their float, the events.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) floating: Floating,
    /// The application's bus, given with [`DockSpace::with_bus`].
    #[cfg(feature = "bus")]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) bus: Option<crate::bus::SpaceBus<Tab>>,
}

/// Where a tab came from before it moved out to a float: its dock, and the tabs next to it in
/// its leaf, so that closing the float can put it back beside them
/// ([`DockSpace::record_origin`]). Tabs are named by their [`TabViewer::id`](crate::TabViewer::id).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub struct TabOrigin {
    /// The tab.
    pub tab: Id,
    /// The dock it was in.
    pub dock: String,
    /// The tab before it in its leaf, if any.
    pub before: Option<Id>,
    /// The tab after it in its leaf, if any.
    pub after: Option<Id>,
}

impl TabOrigin {
    /// The origin of `tab`: the dock `dock`, between `before` and `after`.
    pub fn new(tab: Id, dock: impl Into<String>, before: Option<Id>, after: Option<Id>) -> Self {
        Self {
            tab,
            dock: dock.into(),
            before,
            after,
        }
    }
}

/// What an application declared no tab may be dropped in ([`DockSpace::no_drop_zone`]): rects,
/// and widgets whose rects egui knows.
#[derive(Clone, Debug, Default)]
pub(crate) struct NoDrop {
    pub(crate) rects: Vec<Rect>,
    pub(crate) ids: Vec<Id>,
    /// Folded zones' strips taking drops for a dock ([`DockSpace::bind`]): the strip, and the
    /// dock's name.
    pub(crate) strips: Vec<StripDrop>,
    /// Collapsed zones' restore bands ([`DockSpace::bind`]), in tree order.
    pub(crate) restores: Vec<RestoreDrop>,
    /// Folded zones and collapsed zones a tab was dropped on, to unfold
    /// ([`DockSpace::take_unfolds`]). Kept across frames until taken.
    pub(crate) unfolds: Vec<ZoneName>,
}

/// A collapsed zone ([`Fold::Collapse`]) a dragged tab may bring back ([`DockSpace::bind`]).
#[derive(Clone, Debug)]
pub(crate) struct RestoreDrop {
    /// The zone, to unfold once a tab is dropped on its band.
    pub(crate) zone: ZoneName,
    /// Where it lay and where it would come back.
    pub(crate) restore: Restore,
    /// The dock a tab released on its band joins.
    pub(crate) dock: String,
}

/// A folded zone's strip taking drops for a dock ([`DockSpace::bind`]).
#[derive(Clone, Debug)]
pub(crate) struct StripDrop {
    pub(crate) rect: Rect,
    /// The dock a tab released there joins.
    pub(crate) dock: String,
    /// The folded zone, to unfold once a tab is dropped there.
    pub(crate) unfold: ZoneName,
}

/// A dock of a [`DockSpace`]: its name, its surface and its rules.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct Dock {
    pub(crate) name: String,
    pub(crate) surface: SurfaceIndex,
    pub(crate) rules: DockRules,
    /// Where it was drawn last, for what counts as outside every dock.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) rect: Option<Rect>,
    /// The viewport it was drawn in last: its rect is in that viewport's points.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) viewport: Option<ViewportId>,
    /// Whether a float of the layout shows it, as the floats were last drawn
    /// ([`FloatsArea`](crate::FloatsArea)).
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) floating: bool,
    /// The layout zone holding it ([`Content::Dock`]), as [`DockSpace::bind`] last read it.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) zone: Option<ZoneName>,
}

/// What a dock of a [`DockSpace`] allows, set when it is added with [`DockSpace::add_dock`] or
/// later with [`DockSpace::set_rules`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(default)
)]
#[non_exhaustive]
pub struct DockRules {
    /// The splits a dropped tab may make in this dock, on top of
    /// [`DockArea::allowed_splits`](crate::DockArea::allowed_splits). By default
    /// [`AllowedSplits::All`]; with [`AllowedSplits::None`], a tab dropped in the dock joins a
    /// leaf's tabs.
    pub splits: AllowedSplits,
    /// Whether a tab dragged out of this dock may become a floating window. By default `true`.
    pub windows: bool,
    /// Whether the dock stays in the space, as an empty drop target, once its last tab leaves.
    /// By default `true`; when `false`, the dock is removed from the space as soon as it is
    /// empty.
    pub keep_when_empty: bool,
    /// Whether a dragged tab may be dropped in this dock. By default `true`; when `false`, the
    /// dock is never a drop target, its own tabs included: a tab released over it goes back
    /// where it was, as over a [no-drop zone](DockSpace::no_drop_zone). Its tabs can still be
    /// dragged out of it.
    pub accept_drops: bool,
}

impl Default for DockRules {
    fn default() -> Self {
        Self {
            splits: AllowedSplits::All,
            windows: true,
            keep_when_empty: true,
            accept_drops: true,
        }
    }
}

impl DockRules {
    /// Sets [`splits`](Self::splits).
    #[inline]
    pub fn splits(mut self, splits: AllowedSplits) -> Self {
        self.splits = splits;
        self
    }

    /// Sets [`windows`](Self::windows).
    #[inline]
    pub fn windows(mut self, windows: bool) -> Self {
        self.windows = windows;
        self
    }

    /// Sets [`keep_when_empty`](Self::keep_when_empty).
    #[inline]
    pub fn keep_when_empty(mut self, keep_when_empty: bool) -> Self {
        self.keep_when_empty = keep_when_empty;
        self
    }

    /// Sets [`accept_drops`](Self::accept_drops).
    #[inline]
    pub fn accept_drops(mut self, accept_drops: bool) -> Self {
        self.accept_drops = accept_drops;
        self
    }
}

impl DockSpace<()> {
    /// The id of a [`DockSpace`] whose [`id`](DockSpace::id) was not set.
    pub fn default_id() -> Id {
        Id::unique("egui_stack::DockSpace")
    }
}

impl<Tab> Default for DockSpace<Tab> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Tab> DockSpace<Tab> {
    /// A space with no dock yet.
    pub fn new() -> Self {
        Self {
            state: DockState::new(vec![]),
            docks: Vec::new(),
            windows_dock: None,
            origins: Vec::new(),
            id: DockSpace::default_id(),
            no_drop: NoDrop::default(),
            floating: Floating::default(),
            #[cfg(feature = "bus")]
            bus: None,
        }
    }

    /// Takes several [`DockState`]s, one per dock, as an application drawing one
    /// [`DockArea`](crate::DockArea) per area had them: each main surface becomes the dock of
    /// that name, with those rules, and their windows become the space's. The translations are
    /// the first state's.
    ///
    /// # Panics
    ///
    /// Panics if two docks have the same name.
    pub fn from_states(
        states: impl IntoIterator<Item = (String, DockRules, DockState<Tab>)>,
    ) -> Self {
        let mut space = Self::new();
        let mut windows = Vec::new();
        for (index, (name, rules, state)) in states.into_iter().enumerate() {
            if index == 0 {
                space.state.translations = state.translations.clone();
            }
            for surface in state.into_surfaces() {
                match surface {
                    Surface::Main(tree) => {
                        space.add_dock_tree(name.clone(), rules, tree);
                    }
                    Surface::Window(tree, window) => windows.push((tree, window)),
                    Surface::Empty => (),
                }
            }
        }
        for (tree, window) in windows {
            let surface = space.state.add_window(vec![]);
            if let Some(Surface::Window(empty, state)) = space.state.get_surface_mut(surface) {
                *empty = tree;
                *state = window;
            }
        }
        space
    }

    /// Sets the id the space's [`DockArea`](crate::DockArea)s share, which keeps their drag. Two
    /// spaces shown at once need different ids. By default it's [`DockSpace::default_id`].
    #[inline]
    pub fn with_id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    /// Sets the id the space's [`DockArea`](crate::DockArea)s share; see [`with_id`](Self::with_id).
    #[inline]
    pub fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    /// The id the space's [`DockArea`](crate::DockArea)s share.
    #[inline]
    pub fn id(&self) -> Id {
        self.id
    }

    /// Adds a dock named `name` holding `tabs` in one leaf (none for an empty dock), and returns
    /// its surface.
    ///
    /// # Panics
    ///
    /// Panics if the space already has a dock of that name.
    pub fn add_dock(
        &mut self,
        name: impl Into<String>,
        rules: DockRules,
        tabs: Vec<Tab>,
    ) -> SurfaceIndex {
        // An empty dock has no leaf: the whole dock is the drop target.
        let tree = if tabs.is_empty() {
            Tree::default()
        } else {
            Tree::new(tabs)
        };
        self.add_dock_tree(name, rules, tree)
    }

    /// Adds a dock named `name` holding `tree`, and returns its surface.
    ///
    /// # Panics
    ///
    /// Panics if the space already has a dock of that name.
    pub fn add_dock_tree(
        &mut self,
        name: impl Into<String>,
        rules: DockRules,
        tree: Tree<Tab>,
    ) -> SurfaceIndex {
        let name = name.into();
        assert!(
            !self.has_dock(&name),
            "the space already has a dock named {name:?}"
        );
        // The main surface is the first dock's, until that dock is removed.
        let main_free = !self
            .docks
            .iter()
            .any(|dock| dock.surface == SurfaceIndex::main());
        let surface = if main_free {
            *self.state.main_surface_mut() = tree;
            SurfaceIndex::main()
        } else {
            self.state.add_docked_surface(tree)
        };
        self.docks.push(Dock {
            name,
            surface,
            rules,
            rect: None,
            viewport: None,
            floating: false,
            zone: None,
        });
        surface
    }

    /// Removes the dock named `name` from the space, and returns its tree.
    pub fn remove_dock(&mut self, name: &str) -> Option<Tree<Tab>> {
        let index = self.dock_index(name)?;
        let dock = self.docks.remove(index);
        match self.state.take_surface(dock.surface)? {
            Surface::Main(tree) => Some(tree),
            _ => None,
        }
    }

    /// Whether the space has a dock named `name`.
    #[inline]
    pub fn has_dock(&self, name: &str) -> bool {
        self.dock_index(name).is_some()
    }

    /// The names of the docks, in the order they were added.
    pub fn dock_names(&self) -> impl Iterator<Item = &str> {
        self.docks.iter().map(|dock| dock.name.as_str())
    }

    /// The surface of the dock named `name`.
    pub fn dock_surface(&self, name: &str) -> Option<SurfaceIndex> {
        self.docks
            .iter()
            .find(|dock| dock.name == name)
            .map(|dock| dock.surface)
    }

    /// The name of the dock whose surface is `surface`; `None` for a floating window.
    pub fn dock_name(&self, surface: SurfaceIndex) -> Option<&str> {
        dock_name(&self.docks, surface)
    }

    /// The tree of the dock named `name`.
    pub fn dock(&self, name: &str) -> Option<&Tree<Tab>> {
        let surface = self.dock_surface(name)?;
        self.state.get_surface(surface)?.node_tree()
    }

    /// The tree of the dock named `name`, mutably.
    pub fn dock_mut(&mut self, name: &str) -> Option<&mut Tree<Tab>> {
        let surface = self.dock_surface(name)?;
        self.state.get_surface_mut(surface)?.node_tree_mut()
    }

    /// The rules of the dock named `name`.
    pub fn rules(&self, name: &str) -> Option<DockRules> {
        self.docks
            .iter()
            .find(|dock| dock.name == name)
            .map(|dock| dock.rules)
    }

    /// Sets the rules of the dock named `name`, for example again after loading a saved space.
    /// Returns `false` if there is no such dock.
    pub fn set_rules(&mut self, name: &str, rules: DockRules) -> bool {
        match self.docks.iter_mut().find(|dock| dock.name == name) {
            Some(dock) => {
                dock.rules = rules;
                true
            }
            None => false,
        }
    }

    /// Declares a rect no dragged tab may be dropped in, for this frame: the application's
    /// toolbar, its status bar, any part of the window that is not a dock. A tab released there
    /// goes back where it was: it does not become a floating window, and no drop preview shows
    /// there. A floating window over the zone still takes the tab.
    ///
    /// The zones hold for one frame: declare them every frame, before or after drawing the docks
    /// (declared after, they take effect a frame later, which a drag does not notice). The space
    /// forgets them once its docks have read them.
    ///
    /// ```
    /// # use egui_stack::{DockRules, DockSpace};
    /// # let mut space = DockSpace::<String>::new();
    /// # let toolbar = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 30.0));
    /// space.no_drop_zone(toolbar);
    /// ```
    pub fn no_drop_zone(&mut self, rect: Rect) {
        self.no_drop.rects.push(rect);
    }

    /// Declares several rects no dragged tab may be dropped in, for this frame; see
    /// [`no_drop_zone`](Self::no_drop_zone).
    pub fn no_drop_zones(&mut self, rects: impl IntoIterator<Item = Rect>) {
        self.no_drop.rects.extend(rects);
    }

    /// Declares a widget, by its [`Id`], no dragged tab may be dropped on, for this frame: its
    /// rect is the one egui knows for it ([`egui::Context::read_response`]), so a panel or a bar
    /// that moves needs no rect of the application's. See [`no_drop_zone`](Self::no_drop_zone).
    pub fn no_drop_widget(&mut self, id: Id) {
        self.no_drop.ids.push(id);
    }

    /// Reads where a [`LayoutArea`](crate::layout::LayoutArea) put its zones this frame, and
    /// makes no-drop zones ([`no_drop_zone`](Self::no_drop_zone)) of those no tab may land in:
    /// a zone with [`Drops::Refuse`], a zone holding no dock ([`Content::App`], or a
    /// [`Content::Dock`] this space has not), and a folded zone's strip. A zone with
    /// [`Content::Empty`] that accepts drops stays outside every dock: a tab released there
    /// becomes a floating window.
    ///
    /// A folded zone with [`Constraints::strip_drops`](crate::layout::Constraints::strip_drops)
    /// that accepts drops, holding a dock of this space, takes them on its strip instead: a tab
    /// released there joins that dock (its focused leaf, else its first), the strip highlighted
    /// meanwhile, unless the dock's rules or the viewer's veto refuse it. A folded split zone
    /// with `strip_drops` takes them for one of the docks it holds, the last active one, else the
    /// first accepting drops. Either way the zone is reported to unfold
    /// ([`take_unfolds`](Self::take_unfolds)), and stays folded until the application unfolds it.
    ///
    /// A collapsed zone ([`Fold::Collapse`]) that accepts drops,
    /// holding a dock of this space that accepts them (for a split, one of its docks, chosen as
    /// for a strip), shows a restore band while a tab is dragged: a thin band at its former
    /// boundary ([`Restore::line`]), [`OverlayFeel::restore_band`](crate::OverlayFeel::restore_band)
    /// wide, on the side it would come back on. Bands on the same boundary stack in tree order.
    /// The band wins over whatever lies under it, no-drop zones included, and only inside
    /// itself; hovered, it previews the rect the zone would take back ([`Restore::rect`]). A tab
    /// released there joins the dock, unless the dock's rules or the viewer's veto refuse it,
    /// and the zone is reported to unfold ([`take_unfolds`](Self::take_unfolds)). Released
    /// anywhere else, or cancelled, the layout is left as it was.
    ///
    /// Call it every frame, after the layout is drawn: its zones take effect for the docks drawn
    /// next, a frame later for those already drawn, which a drag does not notice.
    ///
    /// ```
    /// # use egui_stack::{DockArea, DockSpace, DockRules, TabViewer};
    /// # use egui_stack::layout::{Content, Dir, Drops, Layout, LayoutArea, Zone, ZoneName, ZoneViewer};
    /// # struct Tabs;
    /// # impl TabViewer for Tabs {
    /// #     type Tab = String;
    /// #     fn id(&mut self, tab: &mut String) -> egui::Id { egui::Id::unique(tab.as_str()) }
    /// #     fn title(&mut self, tab: &mut String) -> egui::WidgetText { tab.as_str().into() }
    /// #     fn ui(&mut self, ui: &mut egui::Ui, tab: &mut String) { ui.label(tab.as_str()); }
    /// # }
    /// struct Viewer<'a> {
    ///     space: &'a mut DockSpace<String>,
    /// }
    ///
    /// impl ZoneViewer for Viewer<'_> {
    ///     fn ui(&mut self, ui: &mut egui::Ui, zone: &ZoneName) {
    ///         match zone.0.as_str() {
    ///             "toolbar" => { ui.label("Toolbar"); }
    ///             dock => { DockArea::dock(self.space, dock).show_inside(ui, &mut Tabs); }
    ///         }
    ///     }
    /// }
    ///
    /// let mut space = DockSpace::new();
    /// space.add_dock("centre", DockRules::default(), vec!["Editor".to_owned()]);
    /// let mut layout = Layout::new(Zone::split("root", Dir::Vertical, vec![
    ///     Zone::leaf("toolbar", Content::App),
    ///     Zone::leaf("centre", Content::Dock("centre".into())).with(|c| c.grow = true),
    /// ]));
    /// # egui::__run_test_ui(|ui| {
    /// let laid = LayoutArea::new(&mut layout).show_inside(ui, &mut Viewer { space: &mut space });
    /// space.bind(&laid);
    /// # });
    /// ```
    pub fn bind(&mut self, laid: &Laid) {
        #[cfg(feature = "bus")]
        for event in &laid.events {
            self.emit_change(crate::bus::LayoutChange::of(event));
        }
        for zone in &laid.zones {
            if let Some(Content::Dock(name)) = &zone.content
                && let Some(dock) = self.docks.iter_mut().find(|dock| dock.name == *name)
            {
                dock.zone = Some(zone.name.clone());
            }
            if let (Shown::Collapsed, Some(restore)) = (zone.shown, zone.restore) {
                if let Some(dock) = self.restore_dock(zone) {
                    self.no_drop.restores.push(RestoreDrop {
                        zone: zone.name.clone(),
                        restore,
                        dock,
                    });
                }
                continue;
            }
            let Some(content) = &zone.content else {
                // A folded split zone's strip: a target for one of its docks, else a no-drop
                // zone, as a folded leaf's strip is.
                if let Some(strip) = zone.strip {
                    match self.split_strip_dock(zone) {
                        Some(dock) => self.no_drop.strips.push(StripDrop {
                            rect: strip,
                            dock,
                            unfold: zone.name.clone(),
                        }),
                        None => self.no_drop.rects.push(strip),
                    }
                }
                continue;
            };
            if let Some(strip) = zone.strip {
                let takes = match content {
                    Content::Dock(name) => {
                        zone.strip_drops
                            && zone.drops == Drops::Accept
                            && matches!(zone.shown, Shown::Folded | Shown::AutoFolded)
                            && self.rules(name).is_some_and(|rules| rules.accept_drops)
                    }
                    _ => false,
                };
                match content {
                    Content::Dock(name) if takes => {
                        self.no_drop.strips.push(StripDrop {
                            rect: strip,
                            dock: name.clone(),
                            unfold: zone.name.clone(),
                        });
                    }
                    _ => self.no_drop.rects.push(strip),
                }
            }
            if !matches!(zone.shown, Shown::Open | Shown::Over) {
                continue;
            }
            let no_dock = match content {
                Content::Dock(name) => !self.has_dock(name),
                Content::App => true,
                Content::Empty => false,
            };
            if no_dock || zone.drops == Drops::Refuse {
                self.no_drop.rects.push(zone.rect);
            }
        }
    }

    /// The dock a folded split zone's strip takes drops for, with
    /// [`Constraints::strip_drops`](crate::layout::Constraints::strip_drops): of the docks the
    /// split holds that accept drops, the one holding the focused leaf (the last active), else
    /// the first.
    fn split_strip_dock(&self, zone: &LaidZone) -> Option<String> {
        if !(zone.strip_drops
            && zone.drops == Drops::Accept
            && matches!(zone.shown, Shown::Folded | Shown::AutoFolded))
        {
            return None;
        }
        self.split_dock(zone)
    }

    /// The dock a collapsed zone's restore band takes drops for: the dock a leaf holds, or one
    /// of a split's, chosen as for a strip ([`split_dock`](Self::split_dock)); none when the
    /// zone or the dock refuses drops.
    fn restore_dock(&self, zone: &LaidZone) -> Option<String> {
        if zone.drops != Drops::Accept {
            return None;
        }
        match &zone.content {
            Some(Content::Dock(name)) => self
                .rules(name)
                .is_some_and(|rules| rules.accept_drops)
                .then(|| name.clone()),
            Some(_) => None,
            None => self.split_dock(zone),
        }
    }

    /// Of the docks a split zone holds that accept drops, the one holding the focused leaf (the
    /// last active), else the first.
    fn split_dock(&self, zone: &LaidZone) -> Option<String> {
        let candidates: Vec<&String> = zone
            .docks
            .iter()
            .filter(|name| self.rules(name).is_some_and(|rules| rules.accept_drops))
            .collect();
        let focused = self.state.focused_leaf().map(|path| path.surface);
        candidates
            .iter()
            .find(|name| focused.is_some() && self.dock_surface(name) == focused)
            .or(candidates.first())
            .map(|name| (*name).clone())
    }

    /// The zones a tab was dropped on since the last call, for the application to unfold
    /// ([`Layout::set_folded`](crate::layout::Layout::set_folded)): folded zones, whether they
    /// hold one dock or split into several, through their strip, which takes drops with
    /// [`Constraints::strip_drops`](crate::layout::Constraints::strip_drops); and collapsed zones,
    /// through their restore band ([`bind`](Self::bind)). egui_stack unfolds none of them itself:
    /// an application that keeps a zone folded on a drop leaves it out. With the `bus` feature,
    /// the drop's `stack.dock.drop` ask and its `TabMoved` change name the zone too
    /// (`DropAsk::zone`, `LayoutChange::zone`). Call it every frame, after the layout is drawn.
    pub fn take_unfolds(&mut self) -> Vec<ZoneName> {
        std::mem::take(&mut self.no_drop.unfolds)
    }

    /// Collapses the zones of `layout` that collapse ([`Fold::Collapse`]) and whose docks are all
    /// empty: each holds at least one dock ([`Content::Dock`]), and every dock it holds is a dock
    /// of this space with no tab. Zones hidden, already folded, or that grow are left alone, and
    /// so are those under a zone it collapses. Returns the zones it collapsed, parents first.
    ///
    /// Call it every frame, before the layout is drawn: a dock whose last tab leaves gives its
    /// room to its neighbours, and a tab dropped on its restore band brings it back
    /// ([`bind`](Self::bind), [`take_unfolds`](Self::take_unfolds)). A dock that should stay
    /// collapsible needs [`DockRules::keep_when_empty`], the default.
    pub fn collapse_empty(&self, layout: &mut Layout) -> Vec<ZoneName> {
        fn visit<Tab>(space: &DockSpace<Tab>, zone: &mut Zone, out: &mut Vec<ZoneName>) {
            if zone.state.hidden || zone.state.folded {
                return;
            }
            if zone.constraints.fold == Fold::Collapse && !zone.constraints.grow {
                let mut docks = Vec::new();
                zone.visit(&mut |under| {
                    if let ZoneKind::Leaf {
                        content: Content::Dock(name),
                    } = &under.kind
                    {
                        docks.push(name.clone());
                    }
                });
                let empty = !docks.is_empty()
                    && docks
                        .iter()
                        .all(|name| space.dock(name).is_some_and(Tree::is_empty));
                if empty {
                    zone.state.folded = true;
                    zone.state.over = false;
                    out.push(zone.name.clone());
                    return;
                }
            }
            if let ZoneKind::Split { children, .. } = &mut zone.kind {
                for child in children {
                    visit(space, child, out);
                }
            }
        }
        let mut collapsed = Vec::new();
        visit(self, &mut layout.root, &mut collapsed);
        collapsed
    }

    /// The dock whose [`DockArea`](crate::DockArea) also draws the floating windows: the one
    /// set with [`set_windows_dock`](Self::set_windows_dock) if the space has it, otherwise the
    /// first dock.
    #[deprecated(
        since = "0.28.0",
        note = "legacy floating windows: draw the space's floats with `FloatsArea`, which every dock's tabs move out to"
    )]
    pub fn windows_dock(&self) -> Option<&str> {
        windows_dock(&self.docks, self.windows_dock.as_deref())
    }

    /// Sets which dock's [`DockArea`](crate::DockArea) also draws the floating windows.
    #[deprecated(
        since = "0.28.0",
        note = "legacy floating windows: draw the space's floats with `FloatsArea`, which every dock's tabs move out to"
    )]
    pub fn set_windows_dock(&mut self, name: impl Into<String>) {
        self.windows_dock = Some(name.into());
    }

    /// The state holding every dock's tree and the floating windows: to find a tab, read a
    /// window, or anything a [`DockState`] gives.
    #[inline]
    pub fn state(&self) -> &DockState<Tab> {
        &self.state
    }

    /// The state holding every dock's tree and the floating windows, mutably: to move, close or
    /// focus a tab, open a window, or anything a [`DockState`] does. A dock's surface stays a
    /// dock's: use [`remove_dock`](Self::remove_dock) to remove one.
    #[inline]
    pub fn state_mut(&mut self) -> &mut DockState<Tab> {
        &mut self.state
    }

    /// Returns a new [`DockSpace`] while mapping and filtering the tab type; see
    /// [`DockState::filter_map_tabs`].
    pub fn filter_map_tabs<F, NewTab>(&self, function: F) -> DockSpace<NewTab>
    where
        F: FnMut(&Tab) -> Option<NewTab>,
    {
        DockSpace {
            state: self.state.filter_map_tabs(function),
            docks: self.docks.clone(),
            windows_dock: self.windows_dock.clone(),
            origins: self.origins.clone(),
            id: self.id,
            no_drop: self.no_drop.clone(),
            floating: self.floating.clone(),
            // The bus names tabs by a function of the old tab type.
            #[cfg(feature = "bus")]
            bus: None,
        }
    }

    /// Returns a new [`DockSpace`] while mapping the tab type; see [`DockState::map_tabs`].
    pub fn map_tabs<F, NewTab>(&self, mut function: F) -> DockSpace<NewTab>
    where
        F: FnMut(&Tab) -> NewTab,
    {
        self.filter_map_tabs(move |tab| Some(function(tab)))
    }

    /// Records where the tab at `path` is, in a dock, before it moves out to a float: the
    /// dock and its neighbours in the leaf, named by `id_of` (the viewer's
    /// [`TabViewer::id`](crate::TabViewer::id)). It replaces the tab's previous origin. Returns
    /// it, or `None` when `path` is not a tab of a dock (a tab of a floating window keeps the
    /// origin it had).
    pub fn record_origin(
        &mut self,
        path: TabPath,
        mut id_of: impl FnMut(&mut Tab) -> Id,
    ) -> Option<&TabOrigin> {
        record_origin(
            &mut self.state,
            &self.docks,
            &mut self.origins,
            path,
            &mut id_of,
        )?;
        self.origins.last()
    }

    /// Where the tab `tab` came from, if it moved out to a float and its origin was recorded.
    pub fn origin(&self, tab: Id) -> Option<&TabOrigin> {
        self.origins.iter().find(|origin| origin.tab == tab)
    }

    /// Forgets where the tab `tab` came from (it went back, or closed), and returns it.
    pub fn take_origin(&mut self, tab: Id) -> Option<TabOrigin> {
        let index = self.origins.iter().position(|origin| origin.tab == tab)?;
        Some(self.origins.remove(index))
    }

    /// Every origin recorded.
    pub fn origins(&self) -> &[TabOrigin] {
        &self.origins
    }

    /// Turns each floating window of the space ([`Surface::Window`], as `egui::Window`s) into a
    /// dock of its own and a [`FloatHost::Native`] float showing it, keyed by the dock's name
    /// (`window-1`, `window-2`… skipping names taken). The float's place is the window's last
    /// rect, in points, on no screen in particular ([`Place::screen`] `None`): a
    /// [`Place::repair`] puts it on the first screen. Its minimized flag is kept.
    ///
    /// [`Docked`](crate::Docked) does this when it loads a layout saved before floats (layout
    /// version 1); call it to move a space kept apart from its layout.
    pub fn windows_to_floats(&mut self) -> Vec<Float> {
        let windows: Vec<SurfaceIndex> = self
            .state
            .iter_surfaces_indexed()
            .filter(|(_, surface)| matches!(surface, Surface::Window(..)))
            .map(|(index, _)| index)
            .collect();
        let mut floats = Vec::new();
        let mut number = 0;
        for surface in windows {
            let Some(Surface::Window(tree, window)) = self.state.take_surface(surface) else {
                continue;
            };
            let name = loop {
                number += 1;
                let name = format!("window-{number}");
                if !self.has_dock(&name) {
                    break name;
                }
            };
            let rect = window.shown_rect().unwrap_or_else(|| {
                Rect::from_min_size(
                    window.position().unwrap_or(Place::DEFAULT_POS),
                    Place::DEFAULT_SIZE,
                )
            });
            if let Some(slot) = self.state.get_surface_mut(surface) {
                *slot = Surface::Main(tree);
            }
            self.docks.push(Dock {
                name: name.clone(),
                surface,
                rules: DockRules::default(),
                rect: None,
                viewport: None,
                floating: true,
                zone: None,
            });
            let mut float = Float::dock(name.clone(), name, FloatHost::Native, Place::new(rect));
            float.minimized = window.is_minimized();
            floats.push(float);
        }
        floats
    }

    fn dock_index(&self, name: &str) -> Option<usize> {
        self.docks.iter().position(|dock| dock.name == name)
    }

    /// Moves the tab at `path` out of its dock into a new float, as dragging it outside every
    /// dock does: a dock of its own (`float-N`, which keys the float too), its origin recorded
    /// ([`record_origin`](Self::record_origin), with `id_of` the viewer's
    /// [`TabViewer::id`](crate::TabViewer::id)). The float opens at `rect` (points: in the main
    /// window for [`FloatHost::InWindow`], on the first screen for [`FloatHost::Native`]), hosted
    /// as [`float_host`](Self::float_host) says, at the next
    /// [`FloatsArea::show`](crate::FloatsArea::show). Returns its key, or `None` when there is
    /// no tab at `path`.
    pub fn float_out(
        &mut self,
        path: TabPath,
        rect: Rect,
        mut id_of: impl FnMut(&mut Tab) -> Id,
    ) -> Option<String> {
        self.detach(Detach::Tab(path), rect, &mut id_of)
    }

    /// Moves the whole group (leaf) at `path` out of its dock into a new float, its tabs kept in
    /// order; see [`float_out`](Self::float_out).
    pub fn float_out_group(
        &mut self,
        path: NodePath,
        rect: Rect,
        mut id_of: impl FnMut(&mut Tab) -> Id,
    ) -> Option<String> {
        self.detach(Detach::Group(path), rect, &mut id_of)
    }

    fn detach(
        &mut self,
        source: Detach,
        rect: Rect,
        id_of: &mut dyn FnMut(&mut Tab) -> Id,
    ) -> Option<String> {
        detach(
            &mut self.state,
            &mut self.docks,
            &mut self.origins,
            &mut self.floating,
            source,
            rect,
            None,
            id_of,
        )
    }

    /// Takes over the floats' state of `old`, a space this one replaces: an application that
    /// rebuilds its space from a layout of its own (rather than changing the docks in place
    /// with [`dock_mut`](Self::dock_mut), [`add_dock`](Self::add_dock) and
    /// [`remove_dock`](Self::remove_dock)) calls it on the new space before drawing it, so that
    /// its open native windows, the drag in progress and the events not taken yet go on.
    ///
    /// Carried over: the floats' host, their windows' builders and viewports, the tabs moved
    /// out waiting for their float, the events, the drag in progress (dropped if what it drags
    /// is not at the same place in the new space), and the tabs' origins this space does not
    /// record itself. Not carried over: the id; give the new space the old one's
    /// ([`with_id`](Self::with_id)), as the floats' areas and move handles are keyed by it.
    /// `old` keeps its docks and tabs, and no float state.
    pub fn carry_floats_from(&mut self, old: &mut DockSpace<Tab>) {
        let mut floating = std::mem::take(&mut old.floating);
        if floating
            .session
            .as_ref()
            .is_some_and(|session| !session.source_in(&self.state))
        {
            floating.session = None;
        }
        self.floating = floating;
        for origin in std::mem::take(&mut old.origins) {
            if !self.origins.iter().any(|kept| kept.tab == origin.tab) {
                self.origins.push(origin);
            }
        }
    }

    /// The window events since the last call: floats opened from a tab or a group moved out
    /// of a dock, closed (and where their tabs went), moved by the user, or brought back onto
    /// a screen. They come from [`FloatsArea::show`](crate::FloatsArea::show) and from the
    /// docks' drags; call it once a frame, after drawing.
    pub fn take_float_events(&mut self) -> Vec<FloatEvent> {
        std::mem::take(&mut self.floating.events)
    }

    /// Sets what hosts the floats made from a tab or a group moved out of a dock (released
    /// outside every dock, ejected, or moved to a new window from its menu), once the space's
    /// floats are drawn ([`FloatsArea`](crate::FloatsArea)): [`FloatHost::InWindow`] by
    /// default, [`FloatHost::Native`] for a native window of their own.
    pub fn set_float_host(&mut self, host: FloatHost) {
        self.floating.host = host;
    }

    /// Sets what hosts the floats made from a tab moved out; see
    /// [`set_float_host`](Self::set_float_host).
    #[inline]
    pub fn with_float_host(mut self, host: FloatHost) -> Self {
        self.floating.host = host;
        self
    }

    /// What hosts the floats made from a tab moved out
    /// ([`set_float_host`](Self::set_float_host)).
    pub fn float_host(&self) -> FloatHost {
        self.floating.host
    }

    /// The drag in progress across the space's viewports, if any: what it carries (a tab or a
    /// whole group), the viewport it began in, and the other viewport under the pointer.
    ///
    /// A space has one drag, whichever viewport its docks are drawn in: the main window's, or a
    /// native float's ([`FloatsArea`](crate::FloatsArea)). The viewport it began in keeps the
    /// pointer, even past its edges, and sees the release; every other viewport converts the
    /// pointer to its own points (from the desktop's physical pixels, where the system tells
    /// where windows are), shows where the tab would land, and the move is applied once, by the
    /// viewport the drag began in. Escape cancels it, and so does closing the window under the
    /// pointer. Where window positions are hidden (Wayland), each other window outlines its
    /// docks ([`DockPainter::window_outline`](crate::DockPainter::window_outline)), and the first
    /// that sees the pointer over one of its docks shortly after the release takes the drop.
    pub fn dragging(&self) -> Option<crate::SpaceDrag> {
        self.floating.session.as_ref()?.dragged()
    }

    /// The parts a [`DockArea`](crate::DockArea) showing the dock named `name` borrows.
    pub(crate) fn split_for_area(&mut self, name: &str) -> SpaceParts<'_, Tab> {
        let index = self.dock_index(name);
        let shows_windows = index.is_some()
            && windows_dock(&self.docks, self.windows_dock.as_deref()) == Some(name);
        SpaceParts {
            state: &mut self.state,
            docks: &mut self.docks,
            no_drop: &mut self.no_drop,
            floating: &mut self.floating,
            origins: &mut self.origins,
            #[cfg(feature = "bus")]
            bus: self.bus.as_ref(),
            dock: index,
            shows_windows,
            id: self.id,
        }
    }

    /// The docks, mutably, for the floats' drawing.
    pub(crate) fn docks_mut(&mut self) -> &mut Vec<Dock> {
        &mut self.docks
    }

    /// The docks.
    pub(crate) fn docks(&self) -> &[Dock] {
        &self.docks
    }
}

/// What a [`DockArea`](crate::DockArea) showing a dock of a space borrows of it.
pub(crate) struct SpaceParts<'a, Tab> {
    pub(crate) state: &'a mut DockState<Tab>,
    pub(crate) docks: &'a mut Vec<Dock>,
    pub(crate) no_drop: &'a mut NoDrop,
    pub(crate) floating: &'a mut Floating,
    pub(crate) origins: &'a mut Vec<TabOrigin>,
    #[cfg(feature = "bus")]
    pub(crate) bus: Option<&'a crate::bus::SpaceBus<Tab>>,
    pub(crate) dock: Option<usize>,
    pub(crate) shows_windows: bool,
    pub(crate) id: Id,
}

/// What moves out of a dock to a new float.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Detach {
    /// A tab.
    Tab(TabPath),
    /// A whole group: a leaf and all its tabs.
    Group(NodePath),
}

/// Moves `source` into a new dock (`float-N`), records where its tabs came from (unless they
/// leave another float: they keep the origin they had), and asks for a float showing it at
/// `rect`, `physical` on the desktop when known. Returns the new dock's name, which keys the
/// float.
#[allow(clippy::too_many_arguments)]
pub(crate) fn detach<Tab>(
    state: &mut DockState<Tab>,
    docks: &mut Vec<Dock>,
    origins: &mut Vec<TabOrigin>,
    floating: &mut Floating,
    source: Detach,
    rect: Rect,
    physical: Option<Rect>,
    id_of: &mut dyn FnMut(&mut Tab) -> Id,
) -> Option<String> {
    let (node, tabs) = match source {
        Detach::Tab(path) => (path.node_path(), path.tab.0..path.tab.0 + 1),
        Detach::Group(path) => (path, 0..state.leaf(path).ok()?.tabs.len()),
    };
    let from_float = state.is_window(node.surface)
        || docks
            .iter()
            .any(|dock| dock.surface == node.surface && dock.floating);
    let leaf = state.leaf_mut(node).ok()?;
    if tabs.end > leaf.tabs.len() || tabs.is_empty() {
        return None;
    }
    let ids: Vec<Id> = tabs.clone().map(|i| id_of(&mut leaf.tabs[i])).collect();
    if !from_float {
        for i in tabs {
            let path = TabPath::new(node.surface, node.node, crate::TabIndex(i));
            record_origin(state, docks, origins, path, id_of);
        }
    }
    let name = loop {
        floating.next += 1;
        let name = format!("float-{}", floating.next);
        if !docks.iter().any(|dock| dock.name == name) {
            break name;
        }
    };
    let surface = state.add_docked_surface(Tree::default());
    docks.push(Dock {
        name: name.clone(),
        surface,
        // Emptied, it goes, and its float closes with it.
        rules: DockRules::default().keep_when_empty(false),
        rect: None,
        viewport: None,
        floating: true,
        zone: None,
    });
    match source {
        Detach::Tab(path) => state.move_tab(path, crate::TabDestination::EmptySurface(surface)),
        Detach::Group(path) => state.move_node(path, crate::TabDestination::EmptySurface(surface)),
    }
    floating.pending.push(PendingFloat {
        dock: name.clone(),
        rect,
        physical,
        tabs: ids,
    });
    Some(name)
}

/// Records where the tab at `path` is, in a dock of `docks`, in `origins`: see
/// [`DockSpace::record_origin`]. Returns `None` when `path` is not a tab of a dock.
pub(crate) fn record_origin<Tab>(
    state: &mut DockState<Tab>,
    docks: &[Dock],
    origins: &mut Vec<TabOrigin>,
    path: TabPath,
    id_of: &mut dyn FnMut(&mut Tab) -> Id,
) -> Option<()> {
    let dock = dock_name(docks, path.surface)?.to_owned();
    let leaf = state.leaf_mut(path.node_path()).ok()?;
    let at = path.tab.0;
    let tabs = leaf.tabs_mut();
    if at >= tabs.len() {
        return None;
    }
    let tab = id_of(&mut tabs[at]);
    let before = at.checked_sub(1).map(|i| id_of(&mut tabs[i]));
    let after = tabs.get_mut(at + 1).map(&mut *id_of);
    origins.retain(|origin| origin.tab != tab);
    origins.push(TabOrigin::new(tab, dock, before, after));
    Some(())
}

/// The name of the dock of `docks` whose surface is `surface`.
pub(crate) fn dock_name(docks: &[Dock], surface: SurfaceIndex) -> Option<&str> {
    docks
        .iter()
        .find(|dock| dock.surface == surface)
        .map(|dock| dock.name.as_str())
}

/// The windows dock of `docks`: `set`, if it is one of them, otherwise the first.
fn windows_dock<'a>(docks: &'a [Dock], set: Option<&str>) -> Option<&'a str> {
    set.and_then(|set| docks.iter().find(|dock| dock.name == set))
        .or_else(|| docks.first())
        .map(|dock| dock.name.as_str())
}

/// Removes the docks of `docks` that are empty and not kept, emptying their surfaces in `state`.
pub(crate) fn remove_empty_docks<Tab>(state: &mut DockState<Tab>, docks: &mut Vec<Dock>) {
    docks.retain(|dock| {
        let empty = state
            .get_surface(dock.surface)
            .and_then(Surface::node_tree)
            .is_none_or(Tree::is_empty);
        let removed = empty && !dock.rules.keep_when_empty;
        if removed {
            state.take_surface(dock.surface);
        }
        !removed
    });
}
