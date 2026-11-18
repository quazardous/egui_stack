/// The handles that move a whole tab group (a leaf with all its tabs), set with
/// [`DockArea::group_handles`](crate::DockArea::group_handles). A set of flags: combine them
/// with `|`. By default every handle is on ([`ALL`](Self::ALL)). A floating window's move
/// handle ([`MOVE_WINDOW`](Self::MOVE_WINDOW)) is set here too: it moves the window, not a
/// group.
///
/// A group moved by any handle goes through the same drop zones as a dragged tab, keeps its tabs
/// in order and its active tab, follows the viewer's veto
/// ([`TabViewer::dock_drop_allowed`](crate::TabViewer::dock_drop_allowed)), and Escape cancels
/// its drag. A handle that is off is neither drawn nor reacts.
///
/// [`DockArea::drag_group_by`](crate::DockArea::drag_group_by) and
/// [`DockArea::hidden_tab_bar_drags_group`](crate::DockArea::hidden_tab_bar_drags_group) are
/// handles an application asks for itself; these flags do not turn them off.
///
/// ```
/// # use egui_stack::GroupHandles;
/// // The grip and the menu only: no drag from the empty part of the bar, nor with a modifier.
/// let handles = GroupHandles::GRIP | GroupHandles::MENU;
/// assert!(handles.contains(GroupHandles::GRIP));
/// assert!(!handles.contains(GroupHandles::EMPTY_SPACE));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct GroupHandles(u8);

impl GroupHandles {
    /// No handle: a group moves only tab by tab.
    pub const NONE: Self = Self(0);
    /// The empty part of the tab bar, right of the last tab. A tab bar full of tabs has none.
    pub const EMPTY_SPACE: Self = Self(1);
    /// A grip always present at the end of each tab bar, whatever the number of tabs. Clicked or
    /// opened by the keyboard, it shows the group's Move group menu when [`MENU`](Self::MENU)
    /// is on too.
    pub const GRIP: Self = Self(1 << 1);
    /// A tab dragged with [`DockArea::group_drag_modifiers`](crate::DockArea::group_drag_modifiers)
    /// held (Shift by default) carries its whole group.
    pub const MODIFIER_DRAG: Self = Self(1 << 2);
    /// "Move group" in the tab's context menu (and the grip's): the places the group can go,
    /// a new window and, in a [`DockSpace`](crate::DockSpace), its other docks.
    pub const MENU: Self = Self(1 << 3);
    /// A move handle at the end of a floating window's top tab bar, apart from the group grip:
    /// it moves the window, not a group. On a native float the system moves it
    /// ([`egui::ViewportCommand::StartDrag`]), on a float over the main window the
    /// `egui::Window` moves; the pointer shows [`egui::CursorIcon::Move`] over it. Whether the
    /// move can also dock the window is [`FloatsArea::float_move`](crate::FloatsArea::float_move).
    /// Only floats drawn by a [`FloatsArea`](crate::FloatsArea) have one.
    pub const MOVE_WINDOW: Self = Self(1 << 4);
    /// Every handle.
    pub const ALL: Self = Self(0b1_1111);

    /// Whether every flag of `other` is set.
    #[inline]
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// These flags with `flag` set or cleared.
    #[inline]
    #[must_use]
    pub const fn with(self, flag: Self, on: bool) -> Self {
        if on {
            Self(self.0 | flag.0)
        } else {
            Self(self.0 & !flag.0)
        }
    }

    /// The flags set in both: an application's allowed set narrowed by a user's setting.
    #[inline]
    #[must_use]
    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl Default for GroupHandles {
    fn default() -> Self {
        Self::ALL
    }
}

impl std::ops::BitOr for GroupHandles {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for GroupHandles {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl std::ops::BitAnd for GroupHandles {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl std::ops::BitAndAssign for GroupHandles {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl std::ops::Not for GroupHandles {
    type Output = Self;

    #[inline]
    fn not(self) -> Self {
        Self(!self.0 & Self::ALL.0)
    }
}

#[cfg(test)]
mod tests {
    use super::GroupHandles;

    #[test]
    fn flags_combine_and_clear() {
        let all = GroupHandles::default();
        assert_eq!(all, GroupHandles::ALL);
        let no_grip = all.with(GroupHandles::GRIP, false);
        assert!(!no_grip.contains(GroupHandles::GRIP));
        assert!(no_grip.contains(GroupHandles::EMPTY_SPACE | GroupHandles::MENU));
        assert_eq!(!GroupHandles::ALL, GroupHandles::NONE);
        assert_eq!(
            all & !GroupHandles::MENU,
            GroupHandles::EMPTY_SPACE
                | GroupHandles::GRIP
                | GroupHandles::MODIFIER_DRAG
                | GroupHandles::MOVE_WINDOW
        );
        assert_eq!(
            all.intersection(GroupHandles::GRIP),
            GroupHandles::GRIP,
            "a setting narrows the app's set"
        );
    }
}
