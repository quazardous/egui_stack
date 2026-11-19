//! egui_stack's points on an application's bus ([bus-contract](https://docs.rs/bus-contract)),
//! with the `bus` feature: see [`DockSpace::with_bus`].
//!
//! The points are prefixed by the crate that declares them, `stack.`, so that egui_stack and
//! the application's other libraries share one bus:
//!
//! | Point | Kind | Input → value |
//! |---|---|---|
//! | [`stack.dock.drop`](DockDrop) | request, fail-closed | [`DropAsk`] → `()`; a refusal vetoes the drop |
//! | [`stack.layout.changed`](LayoutChanged) | event | [`LayoutChange`] |
//! | [`stack.window.detached`](WindowDetached) | event | [`DetachedInfo`] |
//! | [`stack.window.close`](WindowClose) | request, fails open | [`CloseAsk`] → [`ClosePolicy`] |
//! | [`stack.window.closed`](WindowClosed) | event | [`ClosedInfo`] |
//! | [`stack.window.repaired`](WindowRepaired) | event | [`RepairedInfo`] |
//!
//! Payloads are plain `Send + Sync` values: tabs are named by the key function given to
//! [`DockSpace::with_bus`], never passed as the application's tab type.
//!
//! - `stack.dock.drop` is asked when a dragged tab's destination changes and again at release,
//!   never once per frame; given a bus, it replaces
//!   [`TabViewer::dock_drop_allowed`](crate::TabViewer::dock_drop_allowed). A listener that
//!   fails refuses the drop (fail-closed).
//! - `stack.window.close` starts from the viewer's
//!   [`close_policy`](crate::TabViewer::close_policy); its listeners may change it. A listener
//!   that fails, or refuses, gives [`ClosePolicy::default`] (fails open), so a broken listener
//!   never keeps a window from closing.

use std::fmt;
use std::sync::{Arc, Mutex};

pub use bus_contract;
use bus_contract::{Bus, Point, PointDecl, PointKind, declare_all};

pub use crate::floats::ClosePolicy;
use crate::{DockSpace, Split};

/// `stack.dock.drop`: a tab, or a group, is about to be dropped. A listener refusing it
/// ([`HookError::refuse`](bus_contract::HookError::refuse)) vetoes the drop, its reason shown on
/// the preview. Fail-closed.
#[derive(Debug)]
pub struct DockDrop;

impl Point for DockDrop {
    const ID: &'static str = "stack.dock.drop";
    const KIND: PointKind = PointKind::Request;
    type Input = DropAsk;
    type Value = ();
}

/// `stack.layout.changed`: the layout or the docks changed.
#[derive(Debug)]
pub struct LayoutChanged;

impl Point for LayoutChanged {
    const ID: &'static str = "stack.layout.changed";
    const KIND: PointKind = PointKind::Event;
    type Input = LayoutChange;
    type Value = ();
}

/// `stack.window.detached`: a float opened from a tab or a group moved out of a dock.
#[derive(Debug)]
pub struct WindowDetached;

impl Point for WindowDetached {
    const ID: &'static str = "stack.window.detached";
    const KIND: PointKind = PointKind::Event;
    type Input = DetachedInfo;
    type Value = ();
}

/// `stack.window.close`: the user closes a float; the value is what becomes of its tabs.
/// Fails open to [`ClosePolicy::default`].
#[derive(Debug)]
pub struct WindowClose;

impl Point for WindowClose {
    const ID: &'static str = "stack.window.close";
    const KIND: PointKind = PointKind::Request;
    type Input = CloseAsk;
    type Value = ClosePolicy;
}

/// `stack.window.closed`: a float closed.
#[derive(Debug)]
pub struct WindowClosed;

impl Point for WindowClosed {
    const ID: &'static str = "stack.window.closed";
    const KIND: PointKind = PointKind::Event;
    type Input = ClosedInfo;
    type Value = ();
}

/// `stack.window.repaired`: a float was brought back onto a screen, or docked back when its
/// screen went.
#[derive(Debug)]
pub struct WindowRepaired;

impl Point for WindowRepaired {
    const ID: &'static str = "stack.window.repaired";
    const KIND: PointKind = PointKind::Event;
    type Input = RepairedInfo;
    type Value = ();
}

/// egui_stack's points, readable before anything is built; [`DockSpace::with_bus`] declares
/// them ([`declare_all`]).
pub const POINTS: &[PointDecl] = &[
    PointDecl::of::<DockDrop>(
        "A dragged tab or group is about to be dropped; a refusal vetoes it.",
    )
    .fail_closed(),
    PointDecl::of::<LayoutChanged>("The layout or the docks changed."),
    PointDecl::of::<WindowDetached>("A float opened from a tab or a group moved out of a dock."),
    PointDecl::of::<WindowClose>("A float is closing: what becomes of its tabs."),
    PointDecl::of::<WindowClosed>("A float closed."),
    PointDecl::of::<WindowRepaired>("A float was brought back onto a screen, or docked back."),
];

/// Where a dragged tab would go, for [`DockDrop`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DropTarget {
    /// A new float.
    NewWindow,
    /// An empty dock, as a whole.
    EmptyDock,
    /// After a leaf's tabs.
    Append,
    /// Into a leaf's tabs, at this index.
    Insert(usize),
    /// Beside a leaf, splitting it.
    Split(Split),
}

/// The input of [`DockDrop`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DropAsk {
    /// The tabs moved, by the space's key function, in order.
    pub tabs: Vec<String>,
    /// Whether a whole group (a leaf and all its tabs) moves.
    pub group: bool,
    /// The dock they leave; `None` for a legacy window.
    pub from: Option<String>,
    /// The dock they enter; `None` for a new float or a legacy window.
    pub to: Option<String>,
    /// Whether they leave a dock a float shows.
    pub from_float: bool,
    /// Whether they enter a dock a float shows.
    pub to_float: bool,
    /// Where in the destination.
    pub target: DropTarget,
    /// The layout zone they are dropped in: the folded zone whose strip, or the collapsed zone
    /// whose restore band, they are released on (the zone
    /// [`DockSpace::take_unfolds`](crate::DockSpace::take_unfolds) reports), else the zone
    /// holding the dock they enter, as [`DockSpace::bind`](crate::DockSpace::bind) last read it.
    /// `None` for a new float, or for a dock no bound layout holds.
    pub zone: Option<String>,
}

/// What changed, for [`LayoutChanged`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Change {
    /// A zone was resized.
    Resized,
    /// A zone went back to its default size.
    Reset,
    /// A zone folded.
    Folded,
    /// A zone unfolded.
    Unfolded,
    /// The solver folded a zone, short of room.
    AutoFolded,
    /// A zone the solver had folded came back.
    AutoUnfolded,
    /// A flyout asked to close.
    FlyoutClosed,
    /// A flyout was resized.
    FlyoutResized,
    /// A tab or a group moved between docks, or inside one.
    TabMoved,
    /// A float opened from a tab or a group moved out.
    Detached,
    /// A float was moved or resized by the user.
    FloatMoved,
    /// A float closed.
    FloatClosed,
    /// A float was brought back onto a screen.
    FloatRepaired,
}

/// The input of [`LayoutChanged`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct LayoutChange {
    /// What changed.
    pub change: Change,
    /// The zone, or the flyout's key, if a zone changed. For [`Change::TabMoved`], the zone the
    /// tab or the group was dropped in, as [`DropAsk::zone`] names it.
    pub zone: Option<String>,
    /// The dock, if a dock changed (the one a tab entered).
    pub dock: Option<String>,
    /// The float's key, if a float changed.
    pub float: Option<String>,
}

impl LayoutChange {
    /// The change a [`LayoutArea`](crate::layout::LayoutArea) reported.
    pub fn of(event: &crate::layout::LayoutEvent) -> Self {
        use crate::layout::LayoutEvent;
        let (change, zone) = match event {
            LayoutEvent::Resized { zone, .. } => (Change::Resized, zone.0.clone()),
            LayoutEvent::Reset { zone } => (Change::Reset, zone.0.clone()),
            LayoutEvent::Folded { zone } => (Change::Folded, zone.0.clone()),
            LayoutEvent::Unfolded { zone } => (Change::Unfolded, zone.0.clone()),
            LayoutEvent::FlyoutClosed { key } => (Change::FlyoutClosed, key.clone()),
            LayoutEvent::FlyoutResized { key, .. } => (Change::FlyoutResized, key.clone()),
            LayoutEvent::AutoFolded { zone } => (Change::AutoFolded, zone.0.clone()),
            LayoutEvent::AutoUnfolded { zone } => (Change::AutoUnfolded, zone.0.clone()),
        };
        let mut change = Self::new(change);
        change.zone = Some(zone);
        change
    }

    /// A change `change`, naming nothing yet.
    pub fn new(change: Change) -> Self {
        Self {
            change,
            zone: None,
            dock: None,
            float: None,
        }
    }
}

/// The input of [`WindowDetached`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DetachedInfo {
    /// The float's key.
    pub key: String,
    /// The dock it shows.
    pub dock: String,
    /// Its tabs, by the space's key function.
    pub tabs: Vec<String>,
    /// Whether it is a native window ([`FloatHost::Native`](crate::layout::FloatHost::Native)).
    pub native: bool,
    /// The screen it opened on, when known.
    pub screen: Option<String>,
}

/// The input of [`WindowClose`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct CloseAsk {
    /// The float's key.
    pub key: String,
    /// The dock it shows.
    pub dock: String,
    /// Its tabs, by the space's key function.
    pub tabs: Vec<String>,
    /// The dock each tab came from, when recorded ([`TabOrigin`](crate::TabOrigin)).
    pub origins: Vec<Option<String>>,
}

/// The input of [`WindowClosed`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ClosedInfo {
    /// The float's key.
    pub key: String,
    /// The dock it showed.
    pub dock: String,
    /// The policy applied; `None` when it closed because its dock emptied or went.
    pub policy: Option<ClosePolicy>,
    /// Each tab moved, by the space's key function, and the dock it went to.
    pub moved: Vec<(String, String)>,
}

/// The input of [`WindowRepaired`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RepairedInfo {
    /// The float's key.
    pub key: String,
    /// The screen it was on, if it named one.
    pub from: Option<String>,
    /// The screen it is on now; `None` when it docked back.
    pub to: Option<String>,
}

/// A space's bus and how it names tabs.
pub(crate) struct SpaceBus<Tab> {
    pub(crate) bus: Arc<dyn Bus>,
    pub(crate) key: fn(&Tab) -> String,
    /// The verdicts of `stack.dock.drop` asked since the last release, by ask: a destination
    /// is asked once while hovered, not once per frame.
    verdicts: Verdicts,
}

/// The verdicts of `stack.dock.drop` kept: each ask, and its refusal's reason.
type Verdicts = Arc<Mutex<Vec<(DropAsk, Option<String>)>>>;

impl<Tab> Clone for SpaceBus<Tab> {
    fn clone(&self) -> Self {
        Self {
            bus: self.bus.clone(),
            key: self.key,
            verdicts: self.verdicts.clone(),
        }
    }
}

impl<Tab> fmt::Debug for SpaceBus<Tab> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpaceBus").finish_non_exhaustive()
    }
}

impl<Tab> SpaceBus<Tab> {
    /// Asks `stack.dock.drop`: the refusal's reason (an empty one when it gave none), or `None`
    /// when allowed. Asked again only when `fresh` (a release) or for an ask not asked since the
    /// last release.
    pub(crate) fn drop_refusal(&self, ask: DropAsk, fresh: bool) -> Option<String> {
        if !fresh
            && let Ok(verdicts) = self.verdicts.lock()
            && let Some((_, verdict)) = verdicts.iter().find(|(asked, _)| *asked == ask)
        {
            return verdict.clone();
        }
        let out = self.bus.alter::<DockDrop>(ask.clone(), ());
        let verdict = out.refused.map(|refusal| refusal.reason);
        if let Ok(mut verdicts) = self.verdicts.lock() {
            verdicts.retain(|(asked, _)| *asked != ask);
            verdicts.push((ask, verdict.clone()));
        }
        verdict
    }

    /// Forgets the verdicts asked: a drag ended.
    pub(crate) fn forget_verdicts(&self) {
        if let Ok(mut verdicts) = self.verdicts.lock() {
            verdicts.clear();
        }
    }

    /// Asks `stack.window.close`, starting from `policy`; a failure or a refusal gives the
    /// default.
    pub(crate) fn close_policy(&self, ask: CloseAsk, policy: ClosePolicy) -> ClosePolicy {
        let out = self.bus.alter::<WindowClose>(ask, policy);
        if out.failures.is_empty() && out.refused.is_none() {
            out.value
        } else {
            ClosePolicy::default()
        }
    }

    /// Emits the event `P`.
    pub(crate) fn emit<P: Point<Value = ()>>(&self, input: P::Input) {
        let _ = self.bus.alter::<P>(input, ());
    }
}

impl<Tab> DockSpace<Tab> {
    /// Gives the space the application's bus: egui_stack's points ([`POINTS`]) are declared on
    /// it, the drop veto and the close policy are asked through it, and the layout and window
    /// events are emitted on it. `key` names a tab in the points' payloads (a stable id, never
    /// the tab itself).
    ///
    /// Given a bus, `stack.dock.drop` replaces
    /// [`TabViewer::dock_drop_allowed`](crate::TabViewer::dock_drop_allowed). The events are
    /// also kept for [`take_float_events`](Self::take_float_events).
    #[must_use]
    pub fn with_bus(mut self, bus: Arc<dyn Bus>, key: fn(&Tab) -> String) -> Self {
        self.set_bus(bus, key);
        self
    }

    /// Gives the space the application's bus; see [`with_bus`](Self::with_bus). Points already
    /// declared on it (another space) are kept.
    pub fn set_bus(&mut self, bus: Arc<dyn Bus>, key: fn(&Tab) -> String) {
        // Declaring twice is refused: a second space on the same bus shares the points.
        let _ = declare_all(&*bus, POINTS);
        self.bus = Some(SpaceBus {
            bus,
            key,
            verdicts: Arc::default(),
        });
    }

    /// The bus given with [`with_bus`](Self::with_bus), if any.
    pub fn bus(&self) -> Option<&Arc<dyn Bus>> {
        self.bus.as_ref().map(|bus| &bus.bus)
    }

    /// Emits `stack.layout.changed`, when the space has a bus.
    pub(crate) fn emit_change(&self, change: LayoutChange) {
        if let Some(bus) = &self.bus {
            bus.emit::<LayoutChanged>(change);
        }
    }
}
