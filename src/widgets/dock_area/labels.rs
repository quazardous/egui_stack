use std::borrow::Cow;

/// Accessible names of the [`DockArea`](crate::DockArea)'s buttons and separators.
///
/// Screen readers and test harnesses (through AccessKit) find these controls by their names. The
/// defaults are English; set your own with [`DockArea::labels`](crate::DockArea::labels) to
/// translate them:
///
/// ```rust
/// # use egui_stack::Labels;
/// let mut labels = Labels::default();
/// labels.close_tab = "Fermer l'onglet".into();
/// labels.add_tab = "Ajouter un onglet".into();
/// ```
///
/// Text shown on screen (menus and tooltips) is set by [`Translations`](crate::Translations).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize),
    serde(default)
)]
#[non_exhaustive]
pub struct Labels {
    /// The close button of a tab.
    pub close_tab: Cow<'static, str>,
    /// The button that adds a tab to a tab bar.
    pub add_tab: Cow<'static, str>,
    /// The button that closes every tab of a tab bar.
    pub close_all_tabs: Cow<'static, str>,
    /// The close-all button while it closes the whole window (with the secondary modifier).
    pub close_window: Cow<'static, str>,
    /// The button that collapses or expands a tab bar's body.
    pub collapse: Cow<'static, str>,
    /// The collapse button while it minimizes the whole window (with the secondary modifier).
    pub minimize_window: Cow<'static, str>,
    /// The button that restores a minimized window.
    pub restore_window: Cow<'static, str>,
    /// The button that shows a hidden tab bar.
    pub show_tab_bar: Cow<'static, str>,
    /// A separator between two nodes, which resizes them.
    pub separator: Cow<'static, str>,
}

impl Labels {
    /// The default English labels.
    pub const ENGLISH: Self = Self {
        close_tab: Cow::Borrowed("Close tab"),
        add_tab: Cow::Borrowed("Add tab"),
        close_all_tabs: Cow::Borrowed("Close all tabs"),
        close_window: Cow::Borrowed("Close window"),
        collapse: Cow::Borrowed("Collapse"),
        minimize_window: Cow::Borrowed("Minimize window"),
        restore_window: Cow::Borrowed("Restore window"),
        show_tab_bar: Cow::Borrowed("Show tab bar"),
        separator: Cow::Borrowed("Resize"),
    };
}

impl Default for Labels {
    fn default() -> Self {
        Self::ENGLISH
    }
}
