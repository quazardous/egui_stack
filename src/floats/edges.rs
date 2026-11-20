//! Resizing a float over the main window by its left and top edges.
//!
//! egui resizes a window by the edges that move its origin (left, top and the corners on them)
//! only while the window is movable. A float without a title bar is not: only its move handle
//! moves it, so that drags on its body and tab bars stay theirs. And egui's edge handles are
//! registered before the window's content, so where the dock's body reaches the left edge, the
//! body takes the drag. The left and top edges, and the corners on them, get handles of their
//! own here, above the content; the right and bottom edges keep egui's.

use egui::accesskit::Role;
use egui::emath::GuiRounding as _;
use egui::{Context, CursorIcon, Id, LayerId, Pos2, Rect, Sense, Ui, UiBuilder, Vec2, WidgetInfo};

/// The handles this module adds: their name, the window's sides they move (-1 the left or top
/// side, 1 the right or bottom one, 0 neither) and their accessible label.
const HANDLES: [(&str, (i8, i8), &str); 5] = [
    ("left", (-1, 0), "Resize window left edge"),
    ("top", (0, -1), "Resize window top edge"),
    ("top_left", (-1, -1), "Resize window top-left corner"),
    ("top_right", (1, -1), "Resize window top-right corner"),
    ("bottom_left", (-1, 1), "Resize window bottom-left corner"),
];

fn handle_id(window: Id, name: &str) -> Id {
    window.with(("egui_stack.edge", name))
}

fn start_id(window: Id) -> Id {
    window.with("egui_stack.edge_start")
}

/// While one of the handles of the window `window` is dragged: the window's new outer rect,
/// its rect when the drag began (kept for the whole drag, so that it does not drift) with the
/// dragged sides moved by the pointer, its size kept between `least` and `most`. `outer` is the
/// window's last outer rect.
pub(super) fn dragged(
    ctx: &Context,
    window: Id,
    outer: Rect,
    least: Vec2,
    most: Option<Vec2>,
) -> Option<Rect> {
    let grab = HANDLES.iter().find_map(|(name, grab, _)| {
        ctx.read_response(handle_id(window, name))
            .is_some_and(|r| r.dragged())
            .then_some(*grab)
    });
    let Some((gx, gy)) = grab else {
        ctx.data_mut(|data| data.remove::<Rect>(start_id(window)));
        return None;
    };
    let start = ctx.data_mut(|data| *data.get_temp_mut_or(start_id(window), outer));
    let delta = ctx.input(|i| i.pointer.total_drag_delta())?;
    let most = most.unwrap_or(Vec2::splat(f32::INFINITY)).max(least);
    let (left, right) = side(
        (start.left(), start.right()),
        gx,
        delta.x,
        (least.x, most.x),
    );
    let (top, bottom) = side(
        (start.top(), start.bottom()),
        gy,
        delta.y,
        (least.y, most.y),
    );
    Some(Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, bottom)).round_ui())
}

/// One axis of a dragged window, `(min, max)` when the drag began: the side `grab` moved by
/// `delta`, the length kept within `(least, most)`, the other side staying.
fn side((min, max): (f32, f32), grab: i8, delta: f32, (least, most): (f32, f32)) -> (f32, f32) {
    match grab {
        g if g < 0 => {
            let length = (max - min - delta).clamp(least, most);
            (max - length, max)
        }
        g if g > 0 => {
            let length = (max - min + delta).clamp(least, most);
            (min, min + length)
        }
        _ => (min, max),
    }
}

/// Adds the handles of the window `window`, just drawn at `outer` in `layer`, above its
/// content.
pub(super) fn handles(ctx: &Context, window: Id, layer: LayerId, outer: Rect) {
    let style = ctx.global_style();
    let edge = style.interaction.resize_grab_radius_side;
    let corner = style.interaction.resize_grab_radius_corner;
    let reach = outer.expand(corner);
    let mut ui = Ui::new(
        ctx.clone(),
        window.with("egui_stack.edges"),
        UiBuilder::new().layer_id(layer).max_rect(reach),
    );
    ui.set_clip_rect(reach);
    // An edge is a strip across it, short of the corners; a corner a square on it, reaching
    // less into the window than out of it, clear of the tab bar's buttons.
    let across = |grab: i8, corner_too: bool, min: f32, max: f32| {
        let out = if corner_too { corner } else { edge };
        match grab {
            g if g < 0 => (min - out, min + edge),
            g if g > 0 => (max - edge, max + out),
            _ => (min + corner, max - corner),
        }
    };
    for (name, (gx, gy), label) in HANDLES {
        let corner_too = gx != 0 && gy != 0;
        let (x0, x1) = across(gx, corner_too, outer.left(), outer.right());
        let (y0, y1) = across(gy, corner_too, outer.top(), outer.bottom());
        let rect = Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1));
        let response = ui.interact(rect, handle_id(window, name), Sense::drag());
        response.widget_info(|| WidgetInfo::labeled(Role::Splitter, true, label));
        if response.hovered() || response.dragged() {
            ctx.set_cursor_icon(match (gx, gy) {
                (0, _) => CursorIcon::ResizeVertical,
                (_, 0) => CursorIcon::ResizeHorizontal,
                (x, y) if x == y => CursorIcon::ResizeNwSe,
                _ => CursorIcon::ResizeNeSw,
            });
        }
    }
}
