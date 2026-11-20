//! Moving a float's window: its move handle ([`GroupHandles::MOVE_WINDOW`]), and docking the
//! whole float while it moves ([`FloatMove::Dock`]).
//!
//! [`GroupHandles::MOVE_WINDOW`]: crate::GroupHandles::MOVE_WINDOW

use egui::{Context, Id, LayerId, Order, Pos2, Rect, StrokeKind, Vec2};

use super::{closed, forget_native, sync_docks};
use crate::dock_space::put_tab;
use crate::layout::Layout;
use crate::tab_viewer::{DockDrop, DropVerdict};
use crate::{DockSpace, NodeIndex, NodePath, Style, TabDestination, TabInsert, TabViewer};

/// What moving a floating window does, by its title bar or its move handle
/// ([`GroupHandles::MOVE_WINDOW`](crate::GroupHandles::MOVE_WINDOW)); set with
/// [`FloatsArea::float_move`](crate::FloatsArea::float_move).
///
/// Moving a window never moves a tab or a group by itself: a tab's drag, the group grip and the
/// group modifier ([`GroupHandles`](crate::GroupHandles)) do that.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[non_exhaustive]
pub enum FloatMove {
    /// The window only moves, with the move cursor, and never docks. The default.
    #[default]
    Move,
    /// As in Visual Studio: while the window moves, the dock under the pointer is highlighted,
    /// and released there, the whole float's tabs join it (the leaf under the pointer, else the
    /// dock's focused or first leaf), after the viewer's veto
    /// ([`TabViewer::dock_drop_allowed`], or `stack.dock.drop` with the `bus` feature); the
    /// float closes. It needs to know where the window is: on a native float the move handle
    /// moves the window itself ([`egui::ViewportCommand::OuterPosition`]) to follow the
    /// pointer, and where the system hides window positions (Wayland) it falls back to
    /// [`Move`](Self::Move). A native window's own title bar is moved by the system, which
    /// tells nothing of the release: it only moves.
    Dock,
}

/// Where a drag moves a floating window besides its move handle and its title bar, which
/// always move it; set with [`FloatsArea::move_from`](crate::FloatsArea::move_from). By default
/// [`BAR`](Self::BAR): only them. Combine the others with `|`:
///
/// ```
/// # use egui_stack::MoveFrom;
/// // Empty space, and Alt+drag anywhere.
/// let from = MoveFrom::EMPTY_SPACE | MoveFrom::modifier(egui::Modifiers::ALT);
/// assert!(from.empty_space());
/// assert_eq!(from.modifiers(), egui::Modifiers::ALT);
/// ```
///
/// A move from anywhere goes as one by the handle: [`FloatMove`] says whether it can dock the
/// window. The move handle stays, named for screen readers, for those who move the window
/// otherwise ([`TabViewer::on_window_handle`](crate::TabViewer::on_window_handle)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct MoveFrom {
    empty_space: bool,
    modifiers: egui::Modifiers,
}

impl MoveFrom {
    /// Only the move handle and the title bar move the window. The default.
    pub const BAR: Self = Self {
        empty_space: false,
        modifiers: egui::Modifiers::NONE,
    };

    /// A drag on any part of the float no widget takes a drag from moves it too: the empty part
    /// of a tab bar (which then no longer drags its group, [`GroupHandles::EMPTY_SPACE`]), the
    /// background of a tab's body. Its content keeps its own drags: a slider, a tab, a scroll
    /// area, a separator.
    ///
    /// [`GroupHandles::EMPTY_SPACE`]: crate::GroupHandles::EMPTY_SPACE
    pub const EMPTY_SPACE: Self = Self {
        empty_space: true,
        modifiers: egui::Modifiers::NONE,
    };

    /// A drag begun with `modifiers` held moves the float from anywhere in it, over its widgets
    /// too, as window managers move a window by Alt+drag or Super+drag. [`Modifiers::NONE`]
    /// turns it off.
    ///
    /// [`Modifiers::NONE`]: egui::Modifiers::NONE
    pub const fn modifier(modifiers: egui::Modifiers) -> Self {
        Self {
            empty_space: false,
            modifiers,
        }
    }

    /// Whether a drag on the float's empty space moves it.
    pub const fn empty_space(self) -> bool {
        self.empty_space
    }

    /// The modifiers that make a drag anywhere in the float move it; [`Modifiers::NONE`] when
    /// none does.
    ///
    /// [`Modifiers::NONE`]: egui::Modifiers::NONE
    pub const fn modifiers(self) -> egui::Modifiers {
        self.modifiers
    }
}

impl std::ops::BitOr for MoveFrom {
    type Output = Self;

    /// Both ways; the modifiers of `rhs` when it has some, else those of `self`.
    fn bitor(self, rhs: Self) -> Self {
        Self {
            empty_space: self.empty_space || rhs.empty_space,
            modifiers: if rhs.modifiers.is_none() {
                self.modifiers
            } else {
                rhs.modifiers
            },
        }
    }
}

/// The id of what moves the float showing the dock `dock` of the space `space` from elsewhere
/// than its bar ([`MoveFrom`]).
pub(crate) fn mover_id(space: Id, dock: &str) -> Id {
    space.with(("egui_stack.float_mover", dock))
}

/// Lays out the mover of a float over `rect` in `ui`, called once before the float's content
/// (`above` unset) and once after it (`above` set); it is laid out by one of them. Below the
/// content, widgets keep their drags and it takes the drags on empty space
/// ([`MoveFrom::EMPTY_SPACE`]); while the modifiers of [`MoveFrom::modifier`] are held, it lies
/// above the content and takes every drag over the float.
pub(crate) fn mover(ui: &egui::Ui, id: Id, rect: Rect, from: MoveFrom, above: bool) {
    if from == MoveFrom::BAR {
        return;
    }
    let ctx = ui.ctx();
    let held =
        !from.modifiers.is_none() && ui.input(|i| i.modifiers.matches_logically(from.modifiers));
    let dragged = ctx.is_being_dragged(id);
    let here = if above {
        held
    } else {
        !held && (from.empty_space || dragged)
    };
    if !here {
        return;
    }
    let response = ui.interact(rect, id, egui::Sense::DRAG);
    if response.dragged() || (response.hovered() && ctx.dragged_id().is_none()) {
        ctx.set_cursor_icon(egui::CursorIcon::Move);
    }
}

/// The id of the move handle of the float showing the dock `dock` of the space `space`.
pub(crate) fn move_handle_id(space: Id, dock: &str) -> Id {
    space.with(("egui_stack.float_move_handle", dock))
}

/// A float's window being moved with [`FloatMove::Dock`].
#[derive(Clone, Debug)]
pub(crate) struct WindowMove {
    /// The float's key.
    pub(crate) key: String,
    /// On a native float moved by its handle: the pointer's offset from the window's outer
    /// corner when the drag began, in physical pixels.
    pub(crate) grab: Option<Vec2>,
    /// On a native float: the pointer on the desktop, in physical pixels.
    pub(crate) desktop: Option<Pos2>,
    /// The dock under the pointer, as last hit.
    pub(crate) target: Option<Target>,
}

impl WindowMove {
    pub(crate) fn new(key: &str) -> Self {
        Self {
            key: key.to_owned(),
            grab: None,
            desktop: None,
            target: None,
        }
    }
}

/// Where a moved float would dock.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Target {
    dock: String,
    /// The leaf its tabs join; `None` for an empty dock.
    node: Option<NodeIndex>,
    /// What is highlighted.
    rect: Rect,
    refused: bool,
}

/// The dock of the main window under `pointer` (its points), other than `from`, and the leaf
/// there.
fn hit<Tab>(
    space: &DockSpace<Tab>,
    from: &str,
    main: egui::ViewportId,
    pointer: Pos2,
) -> Option<(String, Option<NodeIndex>, Rect)> {
    let dock = space.docks().iter().find(|dock| {
        !dock.floating
            && dock.name != from
            && dock.viewport == Some(main)
            && dock.rules.accept_drops
            && dock.rect.is_some_and(|rect| rect.contains(pointer))
    })?;
    let tree = space.dock(&dock.name)?;
    let rect = dock.rect?;
    if tree.is_empty() {
        return Some((dock.name.clone(), None, rect));
    }
    let leaves: Vec<NodeIndex> = tree
        .breadth_first_index_iter()
        .filter(|&node| tree.leaf(node).is_ok())
        .collect();
    let under = leaves
        .iter()
        .copied()
        .find(|&node| tree[node].rect().is_some_and(|r| r.contains(pointer)));
    let node = under
        .or_else(|| tree.focused_leaf().filter(|node| leaves.contains(node)))
        .or_else(|| leaves.first().copied())?;
    let rect = tree[node].rect().unwrap_or(rect);
    Some((dock.name.clone(), Some(node), rect))
}

/// Whether the whole float's dock `from` may not join `to` (at `node`): the viewer's veto, or
/// the bus's. `fresh` asks the bus again rather than taking its last verdict.
fn refused<Tab>(
    space: &DockSpace<Tab>,
    viewer: &impl TabViewer<Tab = Tab>,
    from: &str,
    to: &str,
    node: Option<NodeIndex>,
    fresh: bool,
) -> bool {
    let surface = space
        .docks()
        .iter()
        .find(|dock| dock.name == to)
        .map(|dock| dock.surface);
    let (Some(source), Some(surface)) = (space.dock(from), surface) else {
        return true;
    };
    #[cfg(feature = "bus")]
    if let Some(bus) = &space.bus {
        use crate::bus::{DropAsk, DropTarget};
        let ask = DropAsk {
            tabs: source
                .iter()
                .filter_map(|node| node.tabs())
                .flatten()
                .map(bus.key)
                .collect(),
            group: true,
            from: Some(from.to_owned()),
            to: Some(to.to_owned()),
            from_float: true,
            to_float: false,
            target: if node.is_some() {
                DropTarget::Append
            } else {
                DropTarget::EmptyDock
            },
            zone: space
                .docks()
                .iter()
                .find(|dock| dock.name == to)
                .and_then(|dock| dock.zone.as_ref())
                .map(|zone| zone.0.clone()),
        };
        return bus.drop_refusal(ask, fresh).is_some();
    }
    #[cfg(not(feature = "bus"))]
    let _ = fresh;
    let destination = match node {
        Some(node) => TabDestination::Node(NodePath::new(surface, node), TabInsert::Append),
        None => TabDestination::EmptySurface(surface),
    };
    let drop = DockDrop {
        destination: &destination,
        from: Some(from),
        to: Some(to),
    };
    source
        .iter()
        .filter_map(|node| node.tabs())
        .flatten()
        .any(|tab| matches!(viewer.dock_drop_allowed(tab, &drop), DropVerdict::Refuse(_)))
}

/// Keeps the [`FloatMove::Dock`] move of the float `key` (showing `dock`) up to date: the
/// pointer at `pointer` (the main window's points, `None` when unknown) hits a dock, which is
/// highlighted; `released`, the float docks there unless refused, and the move ends.
#[allow(clippy::too_many_arguments)]
pub(crate) fn track<Tab>(
    ctx: &Context,
    layout: &mut Layout,
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    style: &Option<Style>,
    key: &str,
    dock: &str,
    pointer: Option<Pos2>,
    released: bool,
) {
    let main = ctx.viewport_id();
    let target = pointer
        .and_then(|pointer| hit(space, dock, main, pointer))
        .map(|(name, node, rect)| {
            let was = space
                .floating
                .window_move
                .as_ref()
                .and_then(|moving| moving.target.as_ref())
                .filter(|target| target.dock == name && target.node == node);
            // Asked when the target changes, and again at the release.
            let refused = match was {
                Some(was) if !released => was.refused,
                _ => refused(space, viewer, dock, &name, node, released),
            };
            Target {
                dock: name,
                node,
                rect,
                refused,
            }
        });
    if released {
        space.floating.window_move = None;
        #[cfg(feature = "bus")]
        if let Some(bus) = &space.bus {
            bus.forget_verdicts();
        }
        if let Some(target) = target.filter(|target| !target.refused) {
            dock_whole(layout, space, viewer, key, dock, &target.dock, target.node);
            ctx.request_repaint();
        }
        return;
    }
    if let Some(target) = &target {
        paint(ctx, style, target);
    }
    space
        .floating
        .window_move
        .get_or_insert_with(|| WindowMove::new(key))
        .target = target;
    ctx.request_repaint();
}

/// Highlights where the moved float would dock: the drop target's colour, the zone stroke red
/// where it is refused.
fn paint(ctx: &Context, style: &Option<Style>, target: &Target) {
    let style = style
        .clone()
        .unwrap_or_else(|| Style::from_egui(&ctx.global_style()));
    let painter = ctx.layer_painter(LayerId::new(
        Order::Foreground,
        Id::unique("egui_stack.float_dock_target"),
    ));
    if target.refused {
        painter.rect_stroke(
            target.rect,
            style.overlay.zone_corner_radius,
            egui::Stroke::new(2.0, ctx.global_style().visuals.error_fg_color),
            StrokeKind::Inside,
        );
    } else {
        painter.rect_filled(
            target.rect,
            style.overlay.zone_corner_radius,
            style.overlay.selection_color,
        );
    }
}

/// Docks the whole float `key`: every tab of its dock `dock` joins the dock `to` (its leaf
/// `node`, else its focused or first leaf), in order; the float closes, its place kept.
fn dock_whole<Tab>(
    layout: &mut Layout,
    space: &mut DockSpace<Tab>,
    viewer: &mut impl TabViewer<Tab = Tab>,
    key: &str,
    dock: &str,
    to: &str,
    node: Option<NodeIndex>,
) {
    if layout.close_float(key).is_none() {
        return;
    }
    forget_native(space, key);
    let Some(mut tree) = space.remove_dock(dock) else {
        return;
    };
    let mut tabs: Vec<Tab> = Vec::new();
    for leaf in tree.iter_mut() {
        if let Some(leaf) = leaf.get_leaf_mut() {
            tabs.append(&mut leaf.tabs);
        }
    }
    let mut moved = Vec::new();
    #[cfg_attr(not(feature = "bus"), allow(unused_mut))]
    let mut keys = Vec::new();
    for mut tab in tabs {
        let id = viewer.id(&mut tab);
        // Landed in a dock no float shows: where it came from no longer matters.
        space.take_origin(id);
        #[cfg(feature = "bus")]
        keys.extend(space.bus.as_ref().map(|bus| (bus.key)(&tab)));
        let joined = node.and_then(|node| {
            let tree = space.dock_mut(to)?;
            tree.leaf_mut(node).ok().map(|_| node)
        });
        match joined {
            Some(node) => {
                if let Some(tree) = space.dock_mut(to) {
                    tree[node].append_tab(tab);
                }
            }
            None => {
                // `to` is a dock of the space: the tab cannot come back.
                let _ = put_tab(space, &mut |tab| viewer.id(tab), to, tab, None);
            }
        }
        moved.push((id, to.to_owned()));
    }
    closed(space, key.to_owned(), dock.to_owned(), None, moved, keys);
    sync_docks(layout, space);
}
