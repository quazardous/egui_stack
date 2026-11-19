//! Where the floats over the main window may go: [`WorkArea`].

use egui::{Pos2, Rect, ViewportId};

use crate::layout::{Laid, Shown, ZoneName};

/// Where the floats over the main window ([`FloatHost::InWindow`](crate::layout::FloatHost))
/// may go, and where a tab dragged out of every dock may become one
/// ([`FloatsArea::work_area`](crate::FloatsArea::work_area)).
///
/// By default, [`WorkArea::window`]: the whole window, as without a work area. Otherwise a set
/// of regions, each resolved by egui_stack every frame, so the application computes no
/// geometry:
///
/// - [`space`](Self::space): the whole space as drawn, the root zone of the layout last read by
///   [`DockSpace::bind`](crate::DockSpace::bind), else the union of the docks drawn in the main
///   window;
/// - [`zone`](Self::zone): a zone of the layout, leaf or split, as last read by
///   [`DockSpace::bind`](crate::DockSpace::bind); a zone folded, hidden or collapsed is no
///   region;
/// - [`rect`](Self::rect): a rect of the application's, in the main window's points.
///
/// With regions:
///
/// - a tab or a group released outside every dock and outside every region is refused, as over
///   a [no-drop zone](crate::DockSpace::no_drop_zone) ([`Outside::Cancel`]): it stays where it
///   was. Docks, folded zones' strips and collapsed zones' restore bands keep their own rules;
/// - a float over the main window stays inside one region: the one it overlaps most. While it
///   is moved, it goes to the region under the pointer, if any; over a gap between regions it
///   stays in the one it is in, so it never straddles a gap. It is not resized past its region,
///   and a float larger than its region is shrunk to fit;
/// - a region that shrinks or goes, or a saved layout restored with a float outside every
///   region, brings the float back into its region or the nearest one, reported once as
///   [`FloatEvent::Moved`](crate::FloatEvent::Moved). With no region left (every zone gone),
///   the floats are free, as without a work area.
///
/// With [`Outside::Native`], a tab, a group or a float over the main window released past the
/// work area's edge leaves the window instead: it becomes a native float, opened where it was
/// released (a hybrid tear-out, as in Visual Studio or Blender). Where native windows are not
/// available, it falls back to [`Outside::Cancel`].
///
/// Native floats ([`FloatHost::Native`](crate::layout::FloatHost)) ignore the work area: they
/// are free on the desktop. Their own docks are work area for them: a tab dropped into one is
/// taken, whatever the regions of the main window.
///
/// ```
/// # use egui_stack::{WorkArea, Outside};
/// let area = WorkArea::new().zone("centre").zone("right").outside(Outside::Native);
/// # let _ = area;
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkArea {
    parts: Vec<Part>,
    outside: Outside,
}

/// One region of a [`WorkArea`], resolved each frame.
#[derive(Clone, Debug, PartialEq)]
enum Part {
    Space,
    Zone(ZoneName),
    Rect(Rect),
}

/// What becomes of a tab or a group released outside every region of a [`WorkArea`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Outside {
    /// The release is refused, as over a [no-drop zone](crate::DockSpace::no_drop_zone): the
    /// tab or the group stays where it was.
    #[default]
    Cancel,
    /// A tab, a group or a float over the main window released outside every region becomes
    /// a native float ([`FloatHost::Native`](crate::layout::FloatHost)), opened where it was
    /// released: on release, not while dragged (no window follows the pointer out of the main
    /// window). Inside the regions, the floats stay over the main window, bounded as with
    /// [`Cancel`](Self::Cancel).
    ///
    /// Where native windows are not available
    /// ([`ViewportHost::native_windows`](crate::ViewportHost::native_windows) false: egui's
    /// viewports embedded, on the web or a single-viewport backend), it is
    /// [`Cancel`](Self::Cancel).
    Native,
}

impl WorkArea {
    /// The whole window, unrestricted: the default, as without a work area.
    pub fn window() -> Self {
        Self::default()
    }

    /// A work area to which regions are added ([`space`](Self::space), [`zone`](Self::zone),
    /// [`rect`](Self::rect)); with none, it is the whole window, as [`window`](Self::window).
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds the whole space as drawn: the root zone of the layout last read by
    /// [`DockSpace::bind`](crate::DockSpace::bind), else the union of the space's docks drawn in
    /// the main window.
    #[must_use]
    pub fn space(mut self) -> Self {
        self.parts.push(Part::Space);
        self
    }

    /// Adds the zone `name` of the layout, leaf or split, where
    /// [`DockSpace::bind`](crate::DockSpace::bind) last read it: call `bind` every frame. A zone
    /// folded, hidden or collapsed is no region until it opens again.
    #[must_use]
    pub fn zone(mut self, name: impl Into<String>) -> Self {
        self.parts.push(Part::Zone(ZoneName(name.into())));
        self
    }

    /// Adds `rect`, in the main window's points.
    #[must_use]
    pub fn rect(mut self, rect: Rect) -> Self {
        self.parts.push(Part::Rect(rect));
        self
    }

    /// Sets what becomes of a tab released outside every region; by default
    /// [`Outside::Cancel`].
    #[must_use]
    pub fn outside(mut self, outside: Outside) -> Self {
        self.outside = outside;
        self
    }

    /// Whether it is the whole window: no region added.
    pub fn is_window(&self) -> bool {
        self.parts.is_empty()
    }

    /// What becomes of a tab released outside every region.
    pub(crate) fn outside_rule(&self) -> Outside {
        self.outside
    }
}

/// The zones of the layout last read by [`DockSpace::bind`](crate::DockSpace::bind), for the
/// work area: the root's rect and every zone shown open or over its neighbour.
#[derive(Clone, Debug, Default)]
pub(crate) struct BoundZones {
    pub(crate) root: Option<Rect>,
    pub(crate) open: Vec<(ZoneName, Rect)>,
}

impl BoundZones {
    pub(crate) fn of(laid: &Laid) -> Self {
        Self {
            root: laid.zones.first().map(|zone| zone.rect),
            open: laid
                .zones
                .iter()
                .filter(|zone| matches!(zone.shown, Shown::Open | Shown::Over))
                .map(|zone| (zone.name.clone(), zone.rect))
                .collect(),
        }
    }
}

impl WorkArea {
    /// The regions this frame, in the main window's points: `None` when unrestricted (no
    /// region added, or none left). `docks` are the space's docks drawn in the main window,
    /// floats left out.
    pub(crate) fn regions(
        &self,
        zones: &BoundZones,
        docks: impl Iterator<Item = Rect> + Clone,
    ) -> Option<Vec<Rect>> {
        if self.is_window() {
            return None;
        }
        let regions: Vec<Rect> = self
            .parts
            .iter()
            .filter_map(|part| match part {
                Part::Space => zones
                    .root
                    .or_else(|| docks.clone().reduce(|union, rect| union.union(rect))),
                Part::Zone(name) => zones
                    .open
                    .iter()
                    .find(|(zone, _)| zone == name)
                    .map(|(_, rect)| *rect),
                Part::Rect(rect) => Some(*rect),
            })
            .filter(|rect| rect.is_positive())
            .collect();
        (!regions.is_empty()).then_some(regions)
    }
}

/// The region a float at `rect` goes in: the one under `pointer` while the float is moved, if
/// any; else the one it overlaps most; else the nearest. Moved over a gap, a float clamped
/// into its region last frame overlaps that one most, so it stays there: no flicker at a
/// boundary.
pub(crate) fn region_of(regions: &[Rect], rect: Rect, pointer: Option<Pos2>) -> Option<Rect> {
    if let Some(pointer) = pointer
        && let Some(region) = regions.iter().find(|region| region.contains(pointer))
    {
        return Some(*region);
    }
    let overlap = |region: &Rect| {
        let common = region.intersect(rect);
        if common.is_positive() {
            common.area()
        } else {
            0.0
        }
    };
    regions
        .iter()
        .copied()
        .filter(|region| overlap(region) > 0.0)
        .max_by(|a, b| overlap(a).total_cmp(&overlap(b)))
        .or_else(|| {
            regions.iter().copied().min_by(|a, b| {
                a.distance_sq_to_pos(rect.center())
                    .total_cmp(&b.distance_sq_to_pos(rect.center()))
            })
        })
}

/// Whether `viewport` is the main window's: the work area's regions are its own.
pub(crate) fn main_window(viewport: Option<ViewportId>) -> bool {
    viewport.is_none_or(|viewport| viewport == ViewportId::ROOT)
}

#[cfg(test)]
mod tests {
    use egui::{pos2, vec2};

    use super::*;

    const LEFT: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(100.0, 100.0));
    const RIGHT: Rect = Rect::from_min_max(pos2(150.0, 0.0), pos2(250.0, 100.0));

    #[test]
    fn the_window_has_no_region() {
        assert_eq!(
            WorkArea::window().regions(&BoundZones::default(), std::iter::empty()),
            None
        );
        assert!(WorkArea::new().is_window());
    }

    #[test]
    fn a_zone_not_open_is_no_region_and_none_left_is_unrestricted() {
        let area = WorkArea::new().zone("side");
        assert_eq!(
            area.regions(&BoundZones::default(), std::iter::empty()),
            None
        );
        let zones = BoundZones {
            root: None,
            open: vec![(ZoneName("side".into()), LEFT)],
        };
        assert_eq!(area.regions(&zones, std::iter::empty()), Some(vec![LEFT]));
    }

    #[test]
    fn the_space_without_bind_is_the_union_of_its_docks() {
        let area = WorkArea::new().space();
        assert_eq!(
            area.regions(&BoundZones::default(), [LEFT, RIGHT].into_iter()),
            Some(vec![LEFT.union(RIGHT)])
        );
    }

    #[test]
    fn a_float_moved_goes_to_the_region_under_the_pointer() {
        let float = Rect::from_min_size(pos2(10.0, 10.0), vec2(50.0, 50.0));
        assert_eq!(
            region_of(&[LEFT, RIGHT], float, Some(pos2(200.0, 50.0))),
            Some(RIGHT)
        );
    }

    #[test]
    fn over_a_gap_a_float_stays_in_its_region_without_flicker() {
        // Clamped into RIGHT last frame, the pointer over the gap: it stays in RIGHT, frame
        // after frame.
        let mut float = Rect::from_min_size(pos2(150.0, 10.0), vec2(50.0, 50.0));
        for _ in 0..5 {
            let region = region_of(&[LEFT, RIGHT], float, Some(pos2(125.0, 50.0)));
            assert_eq!(region, Some(RIGHT));
            float = float.translate(vec2(-10.0, 0.0));
            float = float.translate(vec2((RIGHT.left() - float.left()).max(0.0), 0.0));
        }
    }

    #[test]
    fn a_float_in_no_region_goes_to_the_nearest() {
        let float = Rect::from_min_size(pos2(300.0, 10.0), vec2(20.0, 20.0));
        assert_eq!(region_of(&[LEFT, RIGHT], float, None), Some(RIGHT));
        let straddling = Rect::from_min_max(pos2(80.0, 0.0), pos2(160.0, 50.0));
        assert_eq!(region_of(&[LEFT, RIGHT], straddling, None), Some(LEFT));
    }
}
