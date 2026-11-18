//! The one drag of a [`DockSpace`](crate::DockSpace) across viewports.
//!
//! egui keeps each viewport's input apart. While the button is held, the system gives every
//! pointer event to the window the drag began in (the implicit grab of Wayland and X11, the
//! capture on Windows): its pointer keeps following the drag past its edges, and no other window
//! sees the drag or its release. The session carries the drag from the viewport it began in, the
//! source, to the others:
//!
//! - the source keeps the pointer, in its points and, where the system tells where windows are,
//!   in desktop physical pixels;
//! - each other viewport converts the pointer to its own points, tests it against its docks as
//!   drawn in its last pass, paints the drop preview there and reports what a release would do;
//! - the source applies the move once, at its release, from the freshest report of the viewport
//!   under the pointer;
//! - where positions are hidden (Wayland), the other viewports outline their docks, and once
//!   the button is released outside the source, the first viewport that sees the pointer over
//!   one of its docks within a few passes takes the drop.

use std::collections::BTreeMap;

use egui::{Pos2, Rect, ViewportId};

use super::drag_and_drop::TreeComponent;
use crate::layout::ZoneName;
use crate::{NodePath, TabDestination, TabPath};

/// How many passes of the source a drag released outside its window waits for another viewport
/// to see the pointer over one of its docks, where the system hides window positions: about a
/// fifth of a second at 60 frames a second.
pub(crate) const RELEASED_PASSES: u8 = 12;

/// The drag in progress across the viewports of a space.
#[derive(Clone, Debug)]
pub(crate) struct DragSession {
    /// What is dragged: a tab, or a whole group (a leaf).
    pub(crate) src: TreeComponent,
    /// The dragged leaf's rect in the source, whose size a new float takes.
    pub(crate) rect: Rect,
    /// The viewport the drag began in.
    pub(crate) source: ViewportId,
    /// The pointer in the source's points; it follows the drag past the source's edges.
    pub(crate) local: Pos2,
    /// The pointer on the desktop, in physical pixels, where the system tells where the source
    /// is.
    pub(crate) desktop: Option<Pos2>,
    /// Whether the pointer is outside the source's viewport.
    pub(crate) outside: bool,
    /// The source's passes since the drag began.
    pub(crate) pass: u64,
    /// Cancelled (Escape, or the window under the pointer closed): nothing moves at release.
    pub(crate) cancelled: bool,
    /// Released outside the source where positions are hidden: the source's passes left for
    /// another viewport to take it.
    pub(crate) released: Option<u8>,
    /// What each other viewport would do with a release, from its last pass.
    pub(crate) reports: BTreeMap<ViewportId, Report>,
}

/// What a viewport other than the source would do with a release, from its last pass.
#[derive(Clone, Debug)]
pub(crate) struct Report {
    /// The source's pass it was made in.
    pub(crate) stamp: u64,
    /// The viewport's rank: the main window's docks 0, a native float's above, its z order
    /// after: where windows overlap, the highest rank is taken to be in front.
    pub(crate) rank: u32,
    /// What a release does.
    pub(crate) outcome: Outcome,
    /// The zone to unfold once the tab lands (a strip or a restore band of the main window).
    pub(crate) unfold: Option<ZoneName>,
}

/// What a release over a viewport does.
#[derive(Clone, Debug)]
pub(crate) enum Outcome {
    /// The tab or the group lands there.
    Drop(TabDestination),
    /// It may not land there (a veto, a no-drop zone, a dock refusing drops): it stays where it
    /// was.
    Refused,
    /// The pointer is over the viewport but over none of its docks: as over nothing, a new
    /// float.
    Free,
}

impl DragSession {
    /// A session for `src` dragged in `source`.
    /// Whether what is dragged is still in `state`, at the same place.
    pub(crate) fn source_in<Tab>(&self, state: &crate::DockState<Tab>) -> bool {
        match self.src {
            TreeComponent::Surface(surface) => state.get_surface(surface).is_some(),
            TreeComponent::Node(path) => state.leaf(path).is_ok(),
            TreeComponent::Tab(path) => state
                .leaf(path.node_path())
                .is_ok_and(|leaf| path.tab.0 < leaf.tabs.len()),
        }
    }

    pub(crate) fn new(src: TreeComponent, rect: Rect, source: ViewportId, local: Pos2) -> Self {
        Self {
            src,
            rect,
            source,
            local,
            desktop: None,
            outside: false,
            pass: 0,
            cancelled: false,
            released: None,
            reports: BTreeMap::new(),
        }
    }

    /// The report of another viewport a release in the source goes by: the freshest reports
    /// (made during the source's last two passes) of the viewports under the pointer, the
    /// highest rank first. Inside the source, only a native float over the main window counts:
    /// the source, where the drag began, is in front of the rest.
    pub(crate) fn remote(&self, source_rank: u32) -> Option<&Report> {
        self.reports
            .iter()
            .filter(|(viewport, report)| {
                **viewport != self.source
                    && report.stamp + 1 >= self.pass
                    && (self.outside || (source_rank == 0 && report.rank > 0))
            })
            .map(|(_, report)| report)
            .max_by_key(|report| report.rank)
    }

    /// What is dragged, for [`DockSpace::dragging`](crate::DockSpace::dragging).
    pub(crate) fn dragged(&self) -> Option<SpaceDrag> {
        let payload = match self.src {
            TreeComponent::Tab(path) => DragPayload::Tab(path),
            TreeComponent::Node(path) => DragPayload::Group(path),
            TreeComponent::Surface(_) => return None,
        };
        let over = self
            .reports
            .iter()
            .filter(|(_, report)| report.stamp + 1 >= self.pass)
            .max_by_key(|(_, report)| report.rank)
            .map(|(viewport, _)| *viewport)
            .filter(|_| self.outside || self.remote(0).is_some());
        Some(SpaceDrag {
            payload,
            source: self.source,
            over,
            released: self.released.is_some(),
            cancelled: self.cancelled,
        })
    }
}

/// What a [`DockSpace`](crate::DockSpace)'s drag carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DragPayload {
    /// A tab.
    Tab(TabPath),
    /// A whole tab group: a leaf and all its tabs.
    Group(NodePath),
}

/// The drag in progress across the viewports of a [`DockSpace`](crate::DockSpace), from
/// [`DockSpace::dragging`](crate::DockSpace::dragging).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SpaceDrag {
    /// What is dragged.
    pub payload: DragPayload,
    /// The viewport the drag began in.
    pub source: ViewportId,
    /// The other viewport under the pointer, as its last pass saw it, if any.
    pub over: Option<ViewportId>,
    /// Released outside the source where window positions are hidden, and waiting a few
    /// passes for another viewport to see the pointer over one of its docks.
    pub released: bool,
    /// Cancelled (Escape, or the window under the pointer closed): nothing moves at release.
    pub cancelled: bool,
}
