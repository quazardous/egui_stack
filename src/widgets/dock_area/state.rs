use egui::{Context, Id, Pos2, Rect};

use super::{
    DraggedNode, DraggedTab,
    drag_and_drop::{DragData, DragDropState, HoverData},
};
use crate::{NodePath, Style, SurfaceIndex, TabPath};

#[derive(Clone, Debug, Default)]
pub(super) struct State {
    pub drag_start: Option<Pos2>,
    pub last_hover_pos: Option<Pos2>,
    pub dnd: Option<DragDropState>,
    pub window_fade: Option<(f64, SurfaceIndex)>,
    /// The tab dragged during the last frame, if any.
    pub dragged: Option<DraggedTab>,
    /// The tab group (a leaf, all its tabs) dragged during the last frame, if any.
    pub dragged_node: Option<DraggedNode>,
    /// Whether the tab being dragged carries its whole group: decided when its drag starts, by
    /// the group drag modifiers held then.
    pub drag_carries_group: Option<bool>,
    /// The pass the drag was last resolved in: the docks of a space resolve it once a pass.
    pub pass: Option<u64>,
    /// Whether this area's viewport shows, as a target, the drag of a space begun in another
    /// viewport: its pointer and drag start are the session's.
    pub remote: bool,
    /// The tab whose drag was under way at the last pass: its ghost follows the pointer, so its
    /// title is left out of its bar.
    pub tab_in_air: Option<TabPath>,
    /// The tab bars drawn this pass, by leaf, in the area's viewport's points.
    pub tab_bars: Vec<(NodePath, Rect)>,
    /// The widget whose drag was cancelled because its release was lost
    /// (`DockArea::drag_lost`): egui still believes the button held and
    /// would take the drag up again, so it stays stopped until the button is let go.
    pub drag_lost: Option<Id>,
}

impl State {
    #[inline(always)]
    pub(super) fn load(ctx: &Context, id: Id) -> Self {
        ctx.data_mut(|d| d.get_temp(id)).unwrap_or(Self {
            drag_start: None,
            last_hover_pos: None,
            dnd: None,
            window_fade: None,
            dragged: None,
            dragged_node: None,
            drag_carries_group: None,
            pass: None,
            remote: false,
            tab_in_air: None,
            tab_bars: Vec::new(),
            drag_lost: None,
        })
    }

    #[inline(always)]
    pub(super) fn store(self, ctx: &Context, id: Id) {
        ctx.data_mut(|d| d.insert_temp(id, self));
    }

    pub(super) fn reset_drag(&mut self) {
        self.dnd = None;
        self.window_fade = None;
        self.drag_start = None;
        self.drag_carries_group = None;
    }

    pub(super) fn set_drag_and_drop(
        &mut self,
        drag: DragData,
        drop: HoverData,
        ctx: &Context,
        style: &Style,
    ) {
        if !self.is_drag_drop_locked(ctx, style) {
            self.dnd = Some(DragDropState {
                hover: drop,
                drag,
                pointer: ctx.pointer_hover_pos().unwrap_or(Pos2::ZERO),
                locked: None,
                src_in_window: false,
                dst_in_window: false,
            })
        }
    }

    #[inline(always)]
    fn is_drag_drop_locked(&self, ctx: &Context, style: &Style) -> bool {
        self.dnd
            .as_ref()
            .is_some_and(|drag_drop_state| drag_drop_state.is_locked(style, ctx))
    }
}
