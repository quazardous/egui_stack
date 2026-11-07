//! Several named docks sharing one drag: see [`DockSpace`].

use egui::{Id, Rect};

use crate::{AllowedSplits, DockState, Surface, SurfaceIndex, Tree};

/// Several named docks, each with its own tree, and floating windows, sharing one drag: a tab
/// dragged from any dock to any other gets the same drop zones, preview and veto as inside one
/// dock.
///
/// Each dock is drawn by its own [`DockArea::dock`](crate::DockArea::dock) call, inside the
/// [`Ui`](egui::Ui) the application lays out for it (a side panel, the central panel, a flyout),
/// at any point of the frame. The calls share the drag through the space's [`id`](Self::id).
/// One dock, the [windows dock](Self::set_windows_dock), also draws the floating windows.
///
/// The docks are surfaces of one [`DockState`] ([`Surface::Main`]), so a [`TabPath`](crate::TabPath)
/// or a [`TabDestination`](crate::TabDestination) names a place in any of them, and
/// [`DockState::move_tab`] moves a tab between them; [`dock_surface`](Self::dock_surface) and
/// [`dock_name`](Self::dock_name) go between a dock's name and its surface.
///
/// ```
/// # use egui_stack::{AllowedSplits, DockArea, DockRules, DockSpace, TabViewer};
/// # struct Viewer;
/// # impl TabViewer for Viewer {
/// #     type Tab = String;
/// #     fn id(&mut self, tab: &mut String) -> egui::Id { egui::Id::new(tab.as_str()) }
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
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DockSpace<Tab> {
    state: DockState<Tab>,
    docks: Vec<Dock>,
    windows_dock: Option<String>,
    #[cfg_attr(
        feature = "serde",
        serde(skip, default = "DockSpace::<()>::default_id")
    )]
    id: Id,
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
}

impl Default for DockRules {
    fn default() -> Self {
        Self {
            splits: AllowedSplits::All,
            windows: true,
            keep_when_empty: true,
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
            id: DockSpace::default_id(),
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

    /// The dock whose [`DockArea`](crate::DockArea) also draws the floating windows: the one
    /// set with [`set_windows_dock`](Self::set_windows_dock) if the space has it, otherwise the
    /// first dock.
    pub fn windows_dock(&self) -> Option<&str> {
        windows_dock(&self.docks, self.windows_dock.as_deref())
    }

    /// Sets which dock's [`DockArea`](crate::DockArea) also draws the floating windows.
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
            id: self.id,
        }
    }

    /// Returns a new [`DockSpace`] while mapping the tab type; see [`DockState::map_tabs`].
    pub fn map_tabs<F, NewTab>(&self, mut function: F) -> DockSpace<NewTab>
    where
        F: FnMut(&Tab) -> NewTab,
    {
        self.filter_map_tabs(move |tab| Some(function(tab)))
    }

    fn dock_index(&self, name: &str) -> Option<usize> {
        self.docks.iter().position(|dock| dock.name == name)
    }

    /// The parts a [`DockArea`](crate::DockArea) showing the dock named `name` borrows.
    pub(crate) fn split_for_area(
        &mut self,
        name: &str,
    ) -> (&mut DockState<Tab>, &mut Vec<Dock>, Option<usize>, bool, Id) {
        let index = self.dock_index(name);
        let shows_windows = index.is_some() && self.windows_dock() == Some(name);
        (
            &mut self.state,
            &mut self.docks,
            index,
            shows_windows,
            self.id,
        )
    }
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
