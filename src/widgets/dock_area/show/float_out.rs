//! Tabs and groups moved out of a dock into a new float, once a space's floats are drawn.

use egui::{Context, Rect};

#[cfg(feature = "bus")]
use crate::TabDestination;
use crate::dock_area::drag_and_drop::TreeComponent;
use crate::dock_space::{Detach, detach};
use crate::{DockArea, SurfaceIndex, TabViewer};

impl<Tab> DockArea<'_, Tab> {
    /// Emits `stack.layout.changed` for a tab or a group dropped at `destination`, when the
    /// space has a bus.
    #[cfg(feature = "bus")]
    pub(super) fn emit_moved(&self, destination: &TabDestination) {
        let Some(bus) = self.space.as_ref().and_then(|space| space.bus) else {
            return;
        };
        let dock = match *destination {
            TabDestination::Node(path, _) => self.dock_name(path.surface),
            TabDestination::EmptySurface(surface) => self.dock_name(surface),
            TabDestination::Window(_) => None,
        };
        let mut change = crate::bus::LayoutChange::new(crate::bus::Change::TabMoved);
        change.zone = self.drop_zone_of(dock);
        change.dock = dock.map(str::to_owned);
        bus.emit::<crate::bus::LayoutChanged>(change);
    }

    /// Whether a tab moved out of every dock becomes a float of the layout: the area shows a
    /// dock of a space whose floats are drawn ([`FloatsArea`](crate::FloatsArea)).
    pub(super) fn floats_on(&self) -> bool {
        self.space
            .as_ref()
            .is_some_and(|space| space.floating.enabled)
    }

    /// Whether `surface` is a dock a float shows.
    pub(super) fn is_float_dock(&self, surface: SurfaceIndex) -> bool {
        self.space.as_ref().is_some_and(|space| {
            space
                .docks
                .iter()
                .any(|dock| dock.surface == surface && dock.floating)
        })
    }

    /// Moves `source`, a tab or a whole group, into a dock of its own, and asks for a float
    /// showing it at `rect` (this viewport's points): the next
    /// [`FloatsArea::show`](crate::FloatsArea::show) opens it. Where each tab came from is
    /// recorded, unless it leaves another float (it keeps the origin it had).
    pub(super) fn float_out(
        &mut self,
        ctx: &Context,
        source: TreeComponent,
        rect: Rect,
        tab_viewer: &mut impl TabViewer<Tab = Tab>,
    ) {
        let source = match source {
            TreeComponent::Tab(path) => Detach::Tab(path),
            TreeComponent::Node(path) => Detach::Group(path),
            TreeComponent::Surface(_) => return,
        };
        let Some(space) = self.space.as_mut() else {
            return;
        };
        // Where the window would be on the desktop, when the system tells where this one is.
        let ppp = ctx.pixels_per_point();
        let physical = ctx.input(|i| i.viewport().inner_rect).map(|inner| {
            Rect::from_min_size(
                ((inner.min.to_vec2() + rect.min.to_vec2()) * ppp).to_pos2(),
                rect.size() * ppp,
            )
        });
        detach(
            self.dock_state,
            space.docks,
            space.origins,
            space.floating,
            source,
            rect,
            physical,
            &mut |tab| tab_viewer.id(tab),
        );
        ctx.request_repaint();
    }
}
