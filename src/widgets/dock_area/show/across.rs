//! The drag of a space across viewports, as each viewport's area sees it: see
//! [`session`](crate::dock_area::session).

use egui::{Key, Pos2, Rect, Ui};

use super::super::{
    drag_and_drop::{DragData, TreeComponent, make_overlay_painter},
    session::{DragSession, Outcome, RELEASED_PASSES, Report},
    state::State,
};
use crate::dock_space::record_origin;
use crate::layout::ZoneName;
use crate::{
    DockArea, Surface, TabDestination, TabIndex, TabInsert, TabPath, TabViewer, WindowOutlinePaint,
};

/// What an area does with its space's drag this pass.
pub(super) enum Across {
    /// The drag is this viewport's, or there is none: resolved as in one viewport.
    Here,
    /// Another viewport's drag, the pointer over this one: its preview shown here, and what a
    /// release would do reported. With `take`, released where positions are hidden: the drop
    /// is this viewport's if the pointer is over one of its docks.
    Remote { take: bool },
    /// This viewport's drag, the pointer over another viewport that shows the preview.
    Covered,
    /// Settled for this pass: nothing more to resolve.
    Done,
}

impl<Tab> DockArea<'_, Tab> {
    /// Keeps the space's drag session up to date from this viewport's pass: the drag begun
    /// here (`drag_data`) moves the session's pointer, and its release applies what the viewport
    /// under the pointer reported; a drag begun elsewhere gets its pointer converted to this
    /// viewport's points, and `drag_data` set to the session's, for the preview.
    pub(super) fn across(
        &mut self,
        ui: &Ui,
        state: &mut State,
        drag_data: &mut Option<DragData>,
        tab_viewer: &mut impl TabViewer<Tab = Tab>,
    ) -> Across {
        if self.space.as_ref().and_then(|space| space.floating.session.as_ref())
            .is_some_and(|session| !self.source_valid(&session.src))
        {
            if let Some(space) = self.space.as_mut() {
                space.floating.session = None;
            }
            Self::idle(state);
            self.forget_verdicts();
        }
        let Some(space) = self.space.as_mut() else {
            return Across::Here;
        };
        let ctx = ui.ctx();
        let viewport = ctx.viewport_id();
        let rank = space.rank;
        let ppp = ctx.pixels_per_point();
        let (escape, released, bounds, origin, own) = ctx.input(|i| {
            (
                i.key_pressed(Key::Escape),
                i.pointer.primary_released(),
                i.viewport_rect(),
                // Where the viewport is on the desktop, in physical pixels: unknown where the
                // system hides window positions.
                i.viewport().inner_rect.map(|r| r.min.to_vec2() * ppp),
                i.pointer.hover_pos(),
            )
        });

        if let Some(drag) = drag_data.as_ref() {
            // The drag is this viewport's: it keeps the pointer, even past its edges.
            let local = state
                .last_hover_pos
                .or_else(|| ctx.input(|i| i.pointer.latest_pos()))
                .unwrap_or(Pos2::ZERO);
            let others = space
                .docks
                .iter()
                .any(|dock| dock.viewport.is_some_and(|v| v != viewport));
            let session = match &mut space.floating.session {
                Some(session)
                    if session.source == viewport
                        && session.src == drag.src
                        && session.released.is_none() =>
                {
                    session
                }
                slot => slot.insert(DragSession::new(
                    drag.src.clone(),
                    drag.rect,
                    viewport,
                    local,
                )),
            };
            session.pass += 1;
            session.local = local;
            session.desktop = origin.map(|origin| (origin + local.to_vec2() * ppp).to_pos2());
            session.outside = !bounds.contains(local);
            session.cancelled |= escape;
            if session.cancelled {
                *drag_data = None;
                state.dnd = None;
                if released {
                    space.floating.session = None;
                    state.reset_drag();
                    self.forget_verdicts();
                }
                return Across::Done;
            }
            let remote = session.remote(rank).cloned();
            if !released {
                if remote.is_some() {
                    state.dnd = None;
                    return Across::Covered;
                }
                return Across::Here;
            }
            let (src, size) = (session.src.clone(), session.rect.size());
            // Released outside, where no viewport could tell the pointer was over it: the first
            // that sees it over one of its docks within a few passes takes it.
            let wait = remote.is_none() && session.outside && session.desktop.is_none() && others;
            if wait {
                session.released = Some(RELEASED_PASSES);
                state.reset_drag();
                state.dnd = None;
                ctx.request_repaint();
                return Across::Done;
            }
            space.floating.session = None;
            let Some(report) = remote else {
                return Across::Here;
            };
            let rect = Rect::from_min_size(local, size);
            self.release_over(report, src, rect, tab_viewer);
            state.reset_drag();
            state.dnd = None;
            self.forget_verdicts();
            ctx.request_repaint();
            return Across::Done;
        }

        let Some(session) = space.floating.session.as_mut() else {
            Self::idle(state);
            return Across::Here;
        };
        if session.source == viewport {
            // The drag begun here is no longer dragged: let go outside and waiting for another
            // viewport, or aborted (egui stops a drag on Escape).
            return match session.released {
                Some(0) => {
                    // Nobody took it: as over nothing, a new float.
                    let (src, rect) = (
                        session.src.clone(),
                        Rect::from_min_size(session.local, session.rect.size()),
                    );
                    space.floating.session = None;
                    self.release_free(src, rect, tab_viewer);
                    self.forget_verdicts();
                    Across::Done
                }
                Some(left) => {
                    session.released = Some(left - 1);
                    ctx.request_repaint();
                    Across::Here
                }
                None => {
                    space.floating.session = None;
                    Across::Here
                }
            };
        }

        // Another viewport's drag.
        if session.cancelled {
            session.reports.remove(&viewport);
            Self::idle(state);
            return Across::Done;
        }
        let pointer = if session.released.is_some() {
            // Let go: the pointer has come into this window, or not.
            own
        } else {
            session
                .desktop
                .zip(origin)
                .map(|(desktop, origin)| ((desktop.to_vec2() - origin) / ppp).to_pos2())
        };
        let Some(pointer) = pointer.filter(|pointer| bounds.contains(*pointer)) else {
            session.reports.remove(&viewport);
            // Where positions are hidden, each window that could take the drop outlines its
            // docks once the pointer leaves the drag's window.
            let outline =
                session.desktop.is_none() && session.outside && session.released.is_none();
            let src = session.src.clone();
            Self::idle(state);
            if outline {
                self.paint_outlines(ui, &src, tab_viewer);
                ctx.request_repaint();
            }
            return Across::Done;
        };
        let (src, rect, take) = (
            session.src.clone(),
            session.rect,
            session.released.is_some(),
        );
        state.remote = true;
        state.last_hover_pos = Some(pointer);
        state.drag_start.get_or_insert(pointer);
        *drag_data = Some(DragData { src, rect });
        ctx.request_repaint();
        Across::Remote { take }
    }

    /// Leaves another viewport's drag, if this area showed it.
    fn idle(state: &mut State) {
        if std::mem::take(&mut state.remote) {
            state.reset_drag();
            state.dnd = None;
        }
    }

    /// Forgets the bus's verdicts: a drag ended.
    pub(super) fn forget_verdicts(&self) {
        #[cfg(feature = "bus")]
        if let Some(bus) = self.space.as_ref().and_then(|space| space.bus) {
            bus.forget_verdicts();
        }
    }

    /// What a release over this viewport would do, for another viewport's drag: reported to
    /// the session, or, `take`n where positions are hidden, done here.
    pub(super) fn report(
        &mut self,
        ui: &Ui,
        state: &mut State,
        outcome: Outcome,
        unfold: Option<ZoneName>,
        take: bool,
        tab_viewer: &mut impl TabViewer<Tab = Tab>,
    ) {
        let viewport = ui.ctx().viewport_id();
        let Some(space) = self.space.as_mut() else {
            return;
        };
        let rank = space.rank;
        let Some(session) = space.floating.session.as_mut() else {
            return;
        };
        if !take {
            let stamp = session.pass;
            session.reports.insert(
                viewport,
                Report {
                    stamp,
                    rank,
                    outcome,
                    unfold,
                },
            );
            return;
        }
        let (src, rect) = (session.src.clone(), session.rect);
        match outcome {
            // Over none of its docks: another viewport may still take it.
            Outcome::Free => return,
            Outcome::Refused => space.floating.session = None,
            Outcome::Drop(_) => space.floating.session = None,
        }
        let report = Report {
            stamp: 0,
            rank,
            outcome,
            unfold,
        };
        self.release_over(report, src, rect, tab_viewer);
        Self::idle(state);
        self.forget_verdicts();
        ui.ctx().request_repaint();
    }

    /// The release of `src` over what `report` says: the move, asked to the veto again; nothing
    /// where refused; a new float at `rect` over nothing.
    fn release_over(
        &mut self,
        report: Report,
        src: TreeComponent,
        rect: Rect,
        tab_viewer: &mut impl TabViewer<Tab = Tab>,
    ) {
        if !self.source_valid(&src) {
            return;
        }
        match report.outcome {
            Outcome::Drop(destination) => {
                let dock = match destination {
                    TabDestination::Node(path, _) => self.dock_name(path.surface),
                    TabDestination::EmptySurface(surface) => self.dock_name(surface),
                    TabDestination::Window(_) => None,
                }
                .map(str::to_owned);
                if let Some(space) = &mut self.space {
                    space.drop_zone = dock.zip(report.unfold.clone());
                }
                if self
                    .refusal_of(&src, tab_viewer, &destination, true)
                    .is_none()
                    && self.apply_move(src, destination, tab_viewer)
                    && let (Some(zone), Some(space)) = (report.unfold, &mut self.space)
                {
                    space.no_drop.unfolds.push(zone);
                }
            }
            Outcome::Refused => {}
            Outcome::Free => self.release_free(src, rect, tab_viewer),
        }
    }

    /// `src` released over nothing: a new float at `rect` (this viewport's points), if it may
    /// float and the veto allows it; else it stays where it was.
    fn release_free(
        &mut self,
        src: TreeComponent,
        rect: Rect,
        tab_viewer: &mut impl TabViewer<Tab = Tab>,
    ) {
        if !self.floats_on() || !self.source_valid(&src) {
            return;
        }
        let may = match src {
            TreeComponent::Tab(path) => self.may_float(path, tab_viewer),
            TreeComponent::Node(path) => self.node_may_float(path, tab_viewer),
            TreeComponent::Surface(_) => false,
        };
        // Outside the floats' work area: refused, or torn out to a native window.
        let Some(host) = self.release_host(rect.min) else {
            return;
        };
        let destination = TabDestination::Window(rect);
        if may
            && self
                .refusal_of(&src, tab_viewer, &destination, true)
                .is_none()
        {
            self.to_float.push((src, rect, Some(host)));
        }
    }

    /// Whether `src` is still a tab or a leaf of the state.
    pub(super) fn source_valid(&self, src: &TreeComponent) -> bool {
        match *src {
            TreeComponent::Tab(path) => self
                .dock_state
                .leaf(path.node_path())
                .is_ok_and(|leaf| path.tab.0 < leaf.tabs.len()),
            TreeComponent::Node(path) => self.dock_state.leaf(path).is_ok(),
            TreeComponent::Surface(_) => false,
        }
    }

    /// Moves `src`, a tab or a whole leaf, to `destination`, once: unless the source or the
    /// destination is gone (a window closed during the drag). A tab that leaves a dock for a
    /// float's records where it came from; one that lands in a dock no float shows forgets it.
    /// Returns whether it moved.
    pub(super) fn apply_move(
        &mut self,
        src: TreeComponent,
        destination: TabDestination,
        tab_viewer: &mut impl TabViewer<Tab = Tab>,
    ) -> bool {
        if !self.source_valid(&src) {
            return false;
        }
        let to = match destination {
            TabDestination::Node(path, _) => {
                let there = self
                    .dock_state
                    .get_surface(path.surface)
                    .and_then(Surface::node_tree)
                    .is_some_and(|tree| tree.iter().nth(path.node.0).is_some());
                if !there || self.dock_state.leaf(path).is_err() {
                    return false;
                }
                if let TabDestination::Node(_, TabInsert::Insert(index)) = destination
                    && self
                        .dock_state
                        .leaf(path)
                        .is_ok_and(|leaf| index.0 > leaf.tabs.len())
                {
                    return false;
                }
                Some(path.surface)
            }
            TabDestination::EmptySurface(surface) => {
                if self
                    .dock_state
                    .get_surface(surface)
                    .and_then(Surface::node_tree)
                    .is_none()
                {
                    return false;
                }
                Some(surface)
            }
            TabDestination::Window(_) => None,
        };
        if let Some(to) = to {
            self.track_origins(&src, to, tab_viewer);
        }
        #[cfg(feature = "bus")]
        self.emit_moved(&destination);
        match src {
            TreeComponent::Node(path) => self.dock_state.move_node(path, destination),
            TreeComponent::Tab(path) => {
                let count = self.dock_state[path.node_path()].tabs_count();
                self.dock_state.move_tab(path, destination);
                // Unhide the tab bar on the source leaf when a tab is dragged out.
                if self.hidable_tab_bars
                    && let Ok(leaf) = self.dock_state.leaf_mut(path.node_path())
                    && leaf.tabs.len() < count
                    && leaf.tab_bar_hidden
                {
                    leaf.tab_bar_hidden = false;
                }
            }
            TreeComponent::Surface(_) => return false,
        }
        true
    }

    /// The origins of the tabs of `src` moving to the surface `to`: recorded when they leave a
    /// dock for a float's, forgotten when they land in a dock no float shows, kept from float
    /// to float.
    fn track_origins(
        &mut self,
        src: &TreeComponent,
        to: crate::SurfaceIndex,
        tab_viewer: &mut impl TabViewer<Tab = Tab>,
    ) {
        let Some(space) = self.space.as_mut() else {
            return;
        };
        if !space.floating.enabled {
            return;
        }
        let from = src.surface_address();
        let is_float = |surface| {
            space
                .docks
                .iter()
                .any(|dock| dock.surface == surface && dock.floating)
        };
        let from_float = is_float(from) || self.dock_state.is_window(from);
        let to_float = is_float(to);
        if from_float == to_float {
            return;
        }
        let paths: Vec<TabPath> = match *src {
            TreeComponent::Tab(path) => vec![path],
            TreeComponent::Node(path) => (0..self.dock_state[path].tabs_count())
                .map(|i| TabPath::new(path.surface, path.node, TabIndex(i)))
                .collect(),
            TreeComponent::Surface(_) => return,
        };
        if to_float {
            for path in paths {
                record_origin(
                    self.dock_state,
                    space.docks,
                    space.origins,
                    path,
                    &mut |tab| tab_viewer.id(tab),
                );
            }
        } else {
            for path in paths {
                if let Ok(leaf) = self.dock_state.leaf_mut(path.node_path())
                    && let Some(tab) = leaf.tabs.get_mut(path.tab.0)
                {
                    let id = tab_viewer.id(tab);
                    space.origins.retain(|origin| origin.tab != id);
                }
            }
        }
    }

    /// Outlines the docks of this viewport that could take the space's drag of `src`, where
    /// window positions are hidden.
    fn paint_outlines(&self, ui: &Ui, src: &TreeComponent, tab_viewer: &impl TabViewer<Tab = Tab>) {
        let Some(space) = self.space.as_ref() else {
            return;
        };
        if !self.source_valid(src) {
            return;
        }
        let viewport = ui.ctx().viewport_id();
        let style = self.style.as_ref().unwrap();
        let painter = make_overlay_painter(ui);
        for dock in space.docks.iter() {
            if dock.viewport != Some(viewport) || dock.surface == src.surface_address() {
                continue;
            }
            let Some(rect) = dock.rect else {
                continue;
            };
            let refused = !dock.rules.accept_drops
                || self.dock_target(&dock.name).is_none_or(|target| {
                    let destination = match target {
                        TreeComponent::Node(path) => TabDestination::Node(path, TabInsert::Append),
                        TreeComponent::Surface(surface) => TabDestination::EmptySurface(surface),
                        TreeComponent::Tab(path) => {
                            TabDestination::Node(path.node_path(), TabInsert::Append)
                        }
                    };
                    self.refusal_of(src, tab_viewer, &destination, false)
                        .is_some()
                });
            self.painter.0.window_outline(
                &painter,
                &WindowOutlinePaint {
                    rect,
                    refused,
                    style,
                    visuals: ui.visuals(),
                },
            );
        }
    }
}
