use std::{ops::RangeInclusive, sync::Arc};

use egui::{
    Align2, CornerRadius, Galley, NumExt, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2, Visuals,
    epaint::TextShape, vec2,
};

use crate::{
    Split, Style, TabDestination, TabStyle,
    utils::{rect_set_size_centered, rect_stroke_box},
};

/// Paints the parts of a [`DockArea`](crate::DockArea): its tabs, tab bars, separators and drop
/// overlay. Set one with [`DockArea::painter`](crate::DockArea::painter).
///
/// Every method has a default, the painting of [`StylePainter`] from the [`Style`] the part
/// receives, so an implementation overrides only the parts it paints itself. A method overridden
/// can still hand some cases to the default by calling it on [`StylePainter`].
///
/// The dock lays the parts out and takes their input; the painter only paints, except
/// [`tab_width`](Self::tab_width), which measures a tab before it is laid out. The parameter
/// structs are `#[non_exhaustive]`: they may gain fields in minor releases.
///
/// The buttons of the tab bars (add, close all, collapse), the hidden tab bar's handle, the tab
/// bar's scroll bar and the tab bodies are still painted from [`Style`].
pub trait DockPainter {
    /// The width of a tab, padding and close button included. A tab bar filling its width
    /// ([`TabBarStyle::fill_tab_bar`](crate::TabBarStyle::fill_tab_bar)) can still make it
    /// wider.
    fn tab_width(&self, tab: &TabMeasure<'_>) -> f32 {
        let tab_style = tab.tab_style;
        let text_width = tab.title.size().x + 2.0 * tab_style.padding.at_least(0.0);
        tab_style
            .minimum_width
            .unwrap_or(0.0)
            .at_least(text_width + tab.close_width)
    }

    /// A tab bar's background, painted before its tabs.
    fn tab_bar(&self, painter: &Painter, bar: &TabBarPaint<'_>) {
        painter.rect_filled(
            bar.rect,
            bar.style.tab_bar.corner_radius,
            bar.style.tab_bar.bg_fill,
        );
    }

    /// A tab, with its title and its close button if it has one.
    ///
    /// A dragged tab is painted on a layer of its own, moved with the pointer.
    fn tab(&self, painter: &Painter, tab: &TabPaint<'_>) {
        let state = tab.state;
        let selected = state.contains(PartState::SELECTED) || state.contains(PartState::DRAGGED);
        let dragged = state.contains(PartState::DRAGGED);
        let kb_focus = state.contains(PartState::FOCUSED);
        let tab_style = tab.tab_style;
        let look = if (selected && state.contains(PartState::LEAF_FOCUSED)) || dragged {
            if kb_focus {
                &tab_style.focused_with_kb_focus
            } else {
                &tab_style.focused
            }
        } else if selected {
            if kb_focus {
                &tab_style.active_with_kb_focus
            } else {
                &tab_style.active
            }
        } else if state.contains(PartState::HOVERED) {
            &tab_style.hovered
        } else if kb_focus {
            &tab_style.inactive_with_kb_focus
        } else {
            &tab_style.inactive
        };
        let rect = tab.rect;

        // The whole tab first, then its outline over it, so that they do not mix.
        painter.rect_filled(rect, look.corner_radius, look.bg_fill);
        let stroke_rect = rect_stroke_box(rect, 1.0);
        painter.rect_stroke(
            stroke_rect,
            look.corner_radius,
            Stroke::new(1.0_f32, look.outline_color),
            StrokeKind::Inside,
        );
        if !dragged {
            // Joins the tab to its body.
            painter.hline(
                RangeInclusive::new(
                    stroke_rect.min.x + f32::max(look.corner_radius.sw.into(), 1.5),
                    stroke_rect.max.x - f32::max(look.corner_radius.se.into(), 1.5),
                ),
                stroke_rect.bottom(),
                Stroke::new(2.0_f32, look.bg_fill),
            );
        }

        let close_width = if tab.close.is_some() {
            Style::TAB_CLOSE_BUTTON_SIZE.min(tab.style.tab_bar.height)
        } else {
            0.0
        };
        let mut text_rect = rect;
        text_rect.set_width(text_rect.width() - close_width);
        let x_spacing = tab_style.padding.at_least(0.0);
        let text_pos = Align2::CENTER_CENTER.pos_in_rect(&text_rect.shrink2(vec2(x_spacing, 0.0)))
            - tab.title.size() / 2.0;
        painter.add(TextShape::new(
            text_pos,
            Arc::clone(tab.title),
            look.text_color,
        ));

        if let Some(close) = tab.close {
            let buttons = &tab.style.buttons;
            let lit = close.state.contains(PartState::HOVERED)
                || close.state.contains(PartState::FOCUSED);
            if lit {
                let mut corner_radius = look.corner_radius;
                corner_radius.nw = 0;
                corner_radius.sw = 0;
                painter.rect_filled(close.rect, corner_radius, buttons.close_tab_bg_fill);
            }
            let color = if lit {
                buttons.close_tab_active_color
            } else {
                buttons.close_tab_color
            };
            let mut x_rect = close.rect;
            rect_set_size_centered(&mut x_rect, Vec2::splat(Style::TAB_CLOSE_X_SIZE));
            painter.line_segment(
                [x_rect.left_top(), x_rect.right_bottom()],
                Stroke::new(1.0_f32, color),
            );
            painter.line_segment(
                [x_rect.right_top(), x_rect.left_bottom()],
                Stroke::new(1.0_f32, color),
            );
        }

        // The line under the bar, under every tab but the selected one.
        if !dragged && (!selected || tab_style.hline_below_active_tab_name) {
            let px = painter.pixels_per_point().recip();
            painter.hline(
                rect.x_range(),
                tab.bar.bottom() - px,
                (px, tab.style.tab_bar.hline_color),
            );
        }
    }

    /// The part of a tab bar right of its last tab, painted after the tabs. Its rect is empty
    /// when the tabs fill the bar.
    fn tab_bar_tail(&self, painter: &Painter, tail: &TabBarTailPaint<'_>) {
        let px = painter.pixels_per_point().recip();
        painter.hline(
            tail.rect.x_range(),
            tail.rect.bottom() - px,
            (px, tail.style.tab_bar.hline_color),
        );
    }

    /// A separator between two nodes.
    fn separator(&self, painter: &Painter, separator: &SeparatorPaint<'_>) {
        let style = &separator.style.separator;
        let state = separator.state;
        let color = if state.contains(PartState::DRAGGED) {
            style.color_dragged
        } else if state.contains(PartState::HOVERED) || state.contains(PartState::FOCUSED) {
            style.color_hovered
        } else {
            style.color_idle
        };
        painter.rect_filled(separator.rect, CornerRadius::ZERO, color);
    }

    /// While a tab is dragged, the node or surface under the pointer, painted first.
    fn drop_target(&self, painter: &Painter, target: &DropTargetPaint<'_>) {
        let highlight = &target.style.overlay.hovered_leaf_highlight;
        painter.rect(
            target.rect.expand(highlight.expansion),
            highlight.corner_radius,
            highlight.color,
            highlight.stroke,
            StrokeKind::Inside,
        );
    }

    /// One of the drop overlay's buttons ([`OverlayType::Widgets`](crate::OverlayType::Widgets)):
    /// the middle one (no split) or a split's.
    fn drop_button(&self, painter: &Painter, button: &DropButtonPaint<'_>) {
        let visuals = &button.style.overlay;
        let button_stroke = Stroke::new(1.0_f32, visuals.button_color);
        painter.rect_stroke(
            button.rect,
            0.0,
            visuals.button_border_stroke,
            StrokeKind::Inside,
        );
        let rect = button.rect.shrink(button.rect.width() * 0.1);
        painter.rect_stroke(rect, 0.0, button_stroke, StrokeKind::Inside);
        let rim = Rect::from_two_pos(rect.min, rect.lerp_inside(vec2(1.0, 0.1)));
        painter.rect(
            rim,
            0.0,
            visuals.button_color,
            Stroke::NONE,
            StrokeKind::Inside,
        );

        if let Some(split) = button.split {
            for line in DASHED_LINE_ALPHAS.chunks(2) {
                let start = rect.lerp_inside(lerp_vec(split, line[0]));
                let end = rect.lerp_inside(lerp_vec(split, line[1]));
                painter.line_segment([start, end], button_stroke);
            }
        }
        if button.hovered {
            let vertical_alphas = vec2(1.0, 0.5);
            let horizontal_alphas = vec2(0.5, 1.0);
            let rect = match button.split {
                Some(Split::Above) => Rect::from_min_size(rect.min, rect.size() * vertical_alphas),
                Some(Split::Left) => Rect::from_min_size(rect.min, rect.size() * horizontal_alphas),
                Some(Split::Below) => {
                    let min = rect.lerp_inside(lerp_vec(Split::Below, 0.0));
                    Rect::from_min_size(min, rect.size() * vertical_alphas)
                }
                Some(Split::Right) => {
                    let min = rect.lerp_inside(lerp_vec(Split::Right, 0.0));
                    Rect::from_min_size(min, rect.size() * horizontal_alphas)
                }
                None => rect,
            };
            painter.rect_filled(rect, 0.0, visuals.selection_color);
        }
    }

    /// Where a dragged tab would land if released now, or why it may not
    /// ([`TabViewer::drop_allowed`](crate::TabViewer::drop_allowed)).
    fn drop_preview(&self, painter: &Painter, drop: &DropPaint<'_>) {
        let overlay = &drop.style.overlay;
        if let TabDestination::Window(_) = drop.destination {
            painter.rect_stroke(
                drop.rect,
                0.0,
                Stroke::new(overlay.selection_stroke_width, overlay.selection_color),
                StrokeKind::Inside,
            );
        } else if !drop.from_buttons {
            painter.rect_filled(drop.rect, 0.0, overlay.selection_color);
        }

        let Some(refusal) = drop.refusal else {
            return;
        };
        let visuals = drop.visuals;
        let color = visuals.error_fg_color;
        painter.rect(
            drop.rect,
            0.0,
            color.gamma_multiply(0.25),
            Stroke::new(overlay.selection_stroke_width.max(1.0), color),
            StrokeKind::Inside,
        );
        if let Some(reason) = &refusal.reason {
            let margin = vec2(6.0, 4.0);
            let min = refusal.pointer + vec2(16.0, 16.0);
            let frame = Rect::from_min_size(min, reason.size() + 2.0 * margin);
            painter.rect(
                frame,
                visuals.menu_corner_radius,
                visuals.window_fill(),
                Stroke::new(1.0, color),
                StrokeKind::Inside,
            );
            painter.galley(min + margin, Arc::clone(reason), visuals.text_color());
        }
    }
}

/// The painter a [`DockArea`](crate::DockArea) uses unless given another: every part painted
/// from the [`Style`], as before painters existed.
#[derive(Clone, Copy, Debug, Default)]
pub struct StylePainter;

impl DockPainter for StylePainter {}

const DASHED_LINE_ALPHAS: [f32; 8] = [
    0.0625, 0.1875, 0.3125, 0.4375, 0.5625, 0.6875, 0.8125, 0.9375,
];

#[inline(always)]
const fn lerp_vec(split: Split, alpha: f32) -> Vec2 {
    if split.is_top_bottom() {
        vec2(alpha, 0.5)
    } else {
        vec2(0.5, alpha)
    }
}

/// The state of a painted part, as flags: a tab uses all of them, a separator
/// [`HOVERED`](Self::HOVERED), [`PRESSED`](Self::PRESSED), [`FOCUSED`](Self::FOCUSED) and
/// [`DRAGGED`](Self::DRAGGED), a tab's close button [`HOVERED`](Self::HOVERED),
/// [`PRESSED`](Self::PRESSED) and [`FOCUSED`](Self::FOCUSED).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PartState(u8);

impl PartState {
    /// No flag.
    pub const NONE: Self = Self(0);
    /// The tab is its leaf's active one, whose body shows.
    pub const SELECTED: Self = Self(1);
    /// The pointer is over the part.
    pub const HOVERED: Self = Self(1 << 1);
    /// A pointer button is held down on the part.
    pub const PRESSED: Self = Self(1 << 2);
    /// The part has the keyboard focus.
    pub const FOCUSED: Self = Self(1 << 3);
    /// The part is being dragged.
    pub const DRAGGED: Self = Self(1 << 4);
    /// The tab's leaf is the dock's focused one, the last one interacted with
    /// ([`DockState::focused_leaf`](crate::DockState::focused_leaf)).
    pub const LEAF_FOCUSED: Self = Self(1 << 5);

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
}

impl std::ops::BitOr for PartState {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for PartState {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// A tab to measure, for [`DockPainter::tab_width`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct TabMeasure<'a> {
    /// The tab's title, laid out.
    pub title: &'a Arc<Galley>,
    /// The flags known before the tab is laid out: [`PartState::SELECTED`],
    /// [`PartState::DRAGGED`] and [`PartState::LEAF_FOCUSED`].
    pub state: PartState,
    /// The width of the tab's close button, `0.0` without one.
    pub close_width: f32,
    /// The dock's style.
    pub style: &'a Style,
    /// The tab's style, after [`TabViewer::tab_style_override`](crate::TabViewer::tab_style_override).
    pub tab_style: &'a TabStyle,
}

/// A tab bar, for [`DockPainter::tab_bar`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct TabBarPaint<'a> {
    /// The whole bar.
    pub rect: Rect,
    /// How far its tabs are scrolled to the left, in points: `0.0` when not scrolled.
    pub scrolled: f32,
    /// The dock's style.
    pub style: &'a Style,
}

/// The end of a tab bar, for [`DockPainter::tab_bar_tail`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct TabBarTailPaint<'a> {
    /// From the right edge of the last tab to the right of the bar.
    pub rect: Rect,
    /// The bar, inside its margins.
    pub bar: Rect,
    /// The dock's style.
    pub style: &'a Style,
}

/// A tab, for [`DockPainter::tab`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct TabPaint<'a> {
    /// The tab, as wide as [`DockPainter::tab_width`] measured (or wider in a filled bar).
    pub rect: Rect,
    /// Its bar, inside the bar's margins.
    pub bar: Rect,
    /// Its title, laid out.
    pub title: &'a Arc<Galley>,
    /// Its state.
    pub state: PartState,
    /// Its close button, if it shows one.
    pub close: Option<TabClose>,
    /// The dock's style.
    pub style: &'a Style,
    /// The tab's style, after [`TabViewer::tab_style_override`](crate::TabViewer::tab_style_override).
    pub tab_style: &'a TabStyle,
}

/// A tab's close button, in [`TabPaint`].
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TabClose {
    /// Where it is, at the tab's right end.
    pub rect: Rect,
    /// Its state.
    pub state: PartState,
}

/// Which way a separator runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SeparatorAxis {
    /// A vertical separator, between a left and a right node; it moves sideways.
    Vertical,
    /// A horizontal separator, between a node above and one below; it moves up and down.
    Horizontal,
}

/// A separator, for [`DockPainter::separator`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct SeparatorPaint<'a> {
    /// The separator, [`SeparatorStyle::width`](crate::SeparatorStyle::width) thick.
    pub rect: Rect,
    /// Which way it runs.
    pub axis: SeparatorAxis,
    /// Its state.
    pub state: PartState,
    /// The dock's style.
    pub style: &'a Style,
}

/// The node or surface under a dragged tab, for [`DockPainter::drop_target`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct DropTargetPaint<'a> {
    /// The node or surface.
    pub rect: Rect,
    /// The dock's style.
    pub style: &'a Style,
}

/// A drop overlay button, for [`DockPainter::drop_button`].
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct DropButtonPaint<'a> {
    /// The button.
    pub rect: Rect,
    /// The split it makes, `None` for the middle button, which adds the tab to the node.
    pub split: Option<Split>,
    /// Whether the pointer picks this button.
    pub hovered: bool,
    /// The dock's style.
    pub style: &'a Style,
}

/// Where a dragged tab would land, for [`DockPainter::drop_preview`].
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct DropPaint<'a> {
    /// Where the tab would go: the hovered tab or tab bar, the node, the half of it a split
    /// would give, or the new window.
    pub rect: Rect,
    /// The destination.
    pub destination: &'a TabDestination,
    /// Whether the destination was picked on the overlay's buttons, which then already show
    /// it: the default paints no fill but a new window's outline.
    pub from_buttons: bool,
    /// Why the tab may not go there, if refused.
    pub refusal: Option<&'a DropRefusal>,
    /// The dock's style.
    pub style: &'a Style,
    /// The visuals of the [`Ui`](egui::Ui) the dock is in.
    pub visuals: &'a Visuals,
}

/// A refused drop, in [`DropPaint`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DropRefusal {
    /// The reason [`TabViewer::drop_allowed`](crate::TabViewer::drop_allowed) gave, laid out.
    pub reason: Option<Arc<Galley>>,
    /// The pointer's position.
    pub pointer: Pos2,
}
