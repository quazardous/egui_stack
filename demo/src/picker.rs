//! The demo's inspection picker: point at a part of the dock and a side panel says what it
//! is (its surface and node, the tab, its rectangle) and which fields of the dock's [`Style`]
//! drew it, with a Copy button for bug reports. A second view of the panel holds egui's own
//! widget inspection.
//!
//! The hit-test only reads what egui_stack makes public: the [`DockState`]'s nodes and their
//! rectangles, the shares of rows, columns and grids for the separators, [`DropZone::at`] for
//! the edge bands, and the tab buttons' rectangles, which [`TabViewer::on_tab_button`]
//! hands to the demo each frame.
//!
//! [`TabViewer::on_tab_button`]: egui_stack::TabViewer::on_tab_button

use egui::{
    Color32, CornerRadius, CursorIcon, Id, Key, KeyboardShortcut, LayerId, Margin, Modifiers,
    Order, Panel, Pos2, Rect, ScrollArea, Sense, Stroke, StrokeKind, Ui,
};
use egui_stack::{
    AllowedSplits, DockState, DropZone, LinearDir, Node, NodeIndex, Style, Surface, SurfaceIndex,
    TabIndex, TabInteractionStyle, Tree,
};

/// Enters or leaves the picking mode. Not Ctrl+Shift+I or Ctrl+Shift+C: Firefox and Chrome
/// open their developer tools or their element picker on them before a web page sees them
/// (and on F12, Ctrl+Shift+J, K, E and M). Ctrl+Shift+F12 is free in both, and egui-pop's
/// gallery picks with it too.
pub const SHORTCUT: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::F12);

/// The part of the dock a pick is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    /// A tab's button, in a tab bar.
    Tab,
    /// A leaf's tab bar, beside its tabs.
    TabBar,
    /// A leaf's body, where its active tab shows.
    Body,
    /// A separator between the children of a row, a column or a grid.
    Separator,
    /// A floating window, outside its leaves (its title bar, its frame).
    Window,
}

/// What the picker found under the pointer.
#[derive(Clone, Debug, PartialEq)]
pub struct Pick {
    /// The part.
    pub kind: PartKind,
    /// The surface it is on.
    pub surface: SurfaceIndex,
    /// The node: the leaf of a tab, a tab bar or a body, the container of a separator.
    pub node: Option<NodeIndex>,
    /// The tab, for [`PartKind::Tab`].
    pub tab: Option<TabIndex>,
    /// Its rectangle, which the picker outlines.
    pub rect: Rect,
    /// Its description, which Copy puts on the clipboard.
    pub text: String,
}

/// The panel's views.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum View {
    #[default]
    Dock,
    Egui,
}

/// The picker's state.
#[derive(Default)]
pub struct Picker {
    /// Whether the pointer picks: a cross-hair over the dock, a click pins.
    pub picking: bool,
    /// Whether the side panel shows.
    pub open: bool,
    /// The part under the pointer while picking.
    pub hovered: Option<Pick>,
    /// The part the last click pinned.
    pub pinned: Option<Pick>,
    view: View,
    /// Where the dock was last shown, which the picking layer covers.
    dock_rect: Option<Rect>,
}

impl Picker {
    /// Enters or leaves the picking mode; entering opens the panel.
    pub fn toggle(&mut self) {
        self.picking = !self.picking;
        self.hovered = None;
        if self.picking {
            self.open = true;
        }
    }

    /// The toolbar's button.
    pub fn button(&mut self, ui: &mut Ui) {
        let shortcut = ui.ctx().format_shortcut(&SHORTCUT);
        if ui
            .selectable_label(self.picking, "Inspect")
            .on_hover_text(format!(
                "Point at a part of the dock: a panel says what it is and which style fields \
                 drew it; a click pins it, Esc leaves ({shortcut})"
            ))
            .clicked()
        {
            self.toggle();
        }
    }

    /// Handles the shortcut and Esc; call it once a frame, before the panel and the dock.
    pub fn keys(&mut self, ui: &Ui) {
        if ui.input_mut(|input| input.consume_shortcut(&SHORTCUT)) {
            self.toggle();
        } else if self.picking
            && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.picking = false;
            self.hovered = None;
        }
    }

    /// Shows the side panel, if open, in `ui`; call it before the dock.
    pub fn panel(&mut self, ui: &mut Ui) {
        if !self.open {
            return;
        }
        Panel::right("picker")
            .resizable(true)
            .default_size(340.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.view, View::Dock, "Dock");
                    ui.selectable_value(&mut self.view, View::Egui, "egui");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            self.open = false;
                            self.picking = false;
                            self.hovered = None;
                        }
                    });
                });
                ui.separator();
                match self.view {
                    View::Dock => self.dock_view(ui),
                    View::Egui => egui_view(ui),
                }
            });
    }

    fn dock_view(&mut self, ui: &mut Ui) {
        let shown = self.hovered.clone().or_else(|| self.pinned.clone());
        ui.horizontal(|ui| {
            if ui
                .add_enabled(shown.is_some(), egui::Button::new("Copy"))
                .on_hover_text("Copy the description, for a bug report")
                .clicked()
                && let Some(pick) = &shown
            {
                ui.ctx().copy_text(pick.text.clone());
            }
            if !self.picking && ui.button("Pick").clicked() {
                self.toggle();
            }
        });
        if self.picking {
            ui.weak("Point at a part of the dock; a click pins it, Esc leaves.");
        }
        ui.separator();
        ScrollArea::vertical().show(ui, |ui| match &shown {
            Some(pick) => {
                ui.add(egui::Label::new(egui::RichText::new(&pick.text).monospace()).wrap());
            }
            None => {
                ui.weak("Nothing picked yet.");
            }
        });
    }

    /// Picks under the pointer and outlines the pick; call it after the dock, with the rect the
    /// dock was given and the tab buttons' rects of this frame.
    pub fn after_dock(
        &mut self,
        ui: &Ui,
        dock_rect: Rect,
        dock_state: &DockState<String>,
        tab_rects: &[(String, Rect)],
        style: &Style,
    ) {
        let ctx = ui.ctx();
        let covered = self.dock_rect.unwrap_or(dock_rect);
        self.dock_rect = Some(dock_rect);
        if self.picking {
            // A layer above the dock and its windows takes the clicks, so that none reaches
            // them, and shows the cross-hair.
            let response = egui::Area::new(Id::unique("picker_layer"))
                .order(Order::Foreground)
                .fixed_pos(covered.min)
                .show(ctx, |ui| ui.allocate_rect(covered, Sense::click()))
                .inner
                .on_hover_cursor(CursorIcon::Crosshair);
            self.hovered = response
                .hover_pos()
                .and_then(|pos| pick_at(dock_state, tab_rects, style, pos));
            if response.clicked() {
                if let Some(pos) = response.interact_pointer_pos() {
                    self.pinned = pick_at(dock_state, tab_rects, style, pos);
                }
                self.picking = false;
                self.hovered = None;
            }
        }
        let outlined = if self.picking {
            self.hovered.as_ref()
        } else if self.open {
            self.pinned.as_ref()
        } else {
            None
        };
        if let Some(pick) = outlined {
            let painter = ctx.layer_painter(LayerId::new(
                Order::Foreground,
                Id::unique("picker_outline"),
            ));
            let color = Color32::from_rgb(255, 120, 40);
            painter.rect_filled(pick.rect, 0.0, color.gamma_multiply(0.12));
            painter.rect_stroke(pick.rect, 0.0, Stroke::new(2.0, color), StrokeKind::Inside);
        }
    }
}

/// egui's own inspection: debug on hover, then its inspection panel.
fn egui_view(ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let mut debug_on_hover = ctx.debug_on_hover();
    if ui
        .checkbox(&mut debug_on_hover, "Debug on hover")
        .on_hover_text("Outline the widget under the pointer and say how egui laid it out")
        .changed()
    {
        ctx.set_debug_on_hover(debug_on_hover);
    }
    ui.separator();
    ScrollArea::vertical().show(ui, |ui| ctx.inspection_ui(ui));
}

/// The part of the dock at `pos`: a floating window's first, then the main surface's.
pub fn pick_at(
    dock_state: &DockState<String>,
    tab_rects: &[(String, Rect)],
    style: &Style,
    pos: Pos2,
) -> Option<Pick> {
    // Windows are above the main surface; the last one listed wins where they overlap.
    let window = dock_state
        .iter_surfaces_indexed()
        .filter_map(|(index, surface)| match surface {
            Surface::Window(_, state) if state.rect().contains(pos) => Some((index, state.rect())),
            _ => None,
        })
        .last();
    match window {
        Some((surface, rect)) => {
            pick_in_tree(dock_state, surface, tab_rects, style, pos).or_else(|| {
                Some(Pick {
                    kind: PartKind::Window,
                    surface,
                    node: None,
                    tab: None,
                    rect,
                    text: format!(
                        "Part: floating window (title bar or frame)\n\
                         Surface: {surface:?}, window\n\
                         Rect: {}\n\
                         Style: drawn by egui::Window with egui's visuals, not by the dock's \
                         Style",
                        rect_text(rect)
                    ),
                })
            })
        }
        None => pick_in_tree(dock_state, SurfaceIndex::main(), tab_rects, style, pos),
    }
}

fn pick_in_tree(
    dock_state: &DockState<String>,
    surface: SurfaceIndex,
    tab_rects: &[(String, Rect)],
    style: &Style,
    pos: Pos2,
) -> Option<Pick> {
    let tree = &dock_state[surface];
    let nodes = || {
        tree.iter()
            .enumerate()
            .map(|(i, node)| (NodeIndex(i), node))
    };
    let header = |kind: &str, node: NodeIndex| {
        let mut text = format!(
            "Part: {kind}\nSurface: {surface:?}{}\nNode: {node:?}, {}\n",
            if surface.is_main() {
                ", main"
            } else {
                ", window"
            },
            node_text(&tree[node])
        );
        if let Some(parent) = parent_text(tree, node) {
            text += &parent;
            text.push('\n');
        }
        text
    };

    // Separators first: their bands overlap the edges of the leaves beside them.
    for (container, node) in nodes() {
        for (what, rect) in separators(node, style) {
            if rect.contains(pos) {
                let mut text = header(&format!("separator, {what}"), container);
                text += &format!("Rect: {} (with the interaction width)\n", rect_text(rect));
                let s = &style.separator;
                text += &format!(
                    "Style:\n  separator: width {}, extra_interact_width {}, extra {}, \
                     color_idle {}, color_hovered {}, color_dragged {}\n",
                    s.width,
                    s.extra_interact_width,
                    s.extra,
                    color(s.color_idle),
                    color(s.color_hovered),
                    color(s.color_dragged)
                );
                return Some(Pick {
                    kind: PartKind::Separator,
                    surface,
                    node: Some(container),
                    tab: None,
                    rect,
                    text,
                });
            }
        }
    }

    for (index, node) in nodes() {
        let Node::Leaf(leaf) = node else { continue };
        if !leaf.rect.contains(pos) {
            continue;
        }
        let tab_bar = Rect::from_min_max(
            leaf.rect.min,
            egui::pos2(leaf.rect.max.x, leaf.viewport.min.y),
        );

        // A tab: its button's rect, as `on_tab_button` gave it, within the tab bar.
        let tab = tab_rects.iter().find_map(|(title, rect)| {
            let rect = rect.intersect(tab_bar);
            if !rect.contains(pos) {
                return None;
            }
            let tab = leaf.tabs.iter().position(|tab| tab == title)?;
            Some((title, TabIndex(tab), rect))
        });
        if let Some((title, tab, rect)) = tab {
            let focused = dock_state
                .focused_leaf()
                .is_some_and(|path| path.surface == surface && path.node == index);
            let (state, drawn) = if tab != leaf.active {
                ("inactive", &style.tab.inactive)
            } else if focused {
                ("focused", &style.tab.focused)
            } else {
                ("active", &style.tab.active)
            };
            let mut text = header(&format!("tab \"{title}\""), index);
            text += &format!(
                "Tab: {tab:?} of {}, {state}\nRect: {}\nStyle:\n  tab.{state}: {}\n  \
                 tab.hovered: {}\n  tab: spacing {}, padding {}, minimum_width {:?}, \
                 hline_below_active_tab_name {}\n",
                leaf.tabs.len(),
                rect_text(rect),
                tab_style_text(drawn),
                tab_style_text(&style.tab.hovered),
                style.tab.spacing,
                style.tab.padding,
                style.tab.minimum_width,
                style.tab.hline_below_active_tab_name
            );
            return Some(Pick {
                kind: PartKind::Tab,
                surface,
                node: Some(index),
                tab: Some(tab),
                rect,
                text,
            });
        }

        if leaf.viewport.contains(pos) {
            let rect = leaf.viewport;
            let zone = DropZone::at(
                rect,
                pos,
                style.overlay.feel.zone_edge_coverage,
                AllowedSplits::default(),
            );
            let body = &style.tab.tab_body;
            let mut text = header("body", index);
            text += &format!(
                "Rect: {}\nDrop zone here: {zone:?} (edge bands {:.0}% of the body)\nStyle:\n  \
                 tab.tab_body: bg_fill {}, stroke {}, inner_margin {}, corner_radius {}\n  \
                 overlay: zone_fill {}, zone_stroke {}, selection_color {}\n",
                rect_text(rect),
                style.overlay.feel.zone_edge_coverage * 100.0,
                color(body.bg_fill),
                stroke(body.stroke),
                margin(body.inner_margin),
                radius(body.corner_radius),
                color(style.overlay.zone_fill),
                stroke(style.overlay.zone_stroke),
                color(style.overlay.selection_color)
            );
            return Some(Pick {
                kind: PartKind::Body,
                surface,
                node: Some(index),
                tab: None,
                rect,
                text,
            });
        }

        let bar = &style.tab_bar;
        let mut text = header("tab bar", index);
        text += &format!(
            "Rect: {}\nStyle:\n  tab_bar: bg_fill {}, height {}, hline_color {}, inner_margin {}, \
             corner_radius {}, fill_tab_bar {}\n",
            rect_text(tab_bar),
            color(bar.bg_fill),
            bar.height,
            color(bar.hline_color),
            margin(bar.inner_margin),
            radius(bar.corner_radius),
            bar.fill_tab_bar
        );
        return Some(Pick {
            kind: PartKind::TabBar,
            surface,
            node: Some(index),
            tab: None,
            rect: tab_bar,
            text,
        });
    }
    None
}

/// The separators of a container, each with what it is and its band, as wide as it reacts to
/// the pointer. They stand at the shares' boundaries; a column with collapsed leaves places
/// them otherwise, which the shares do not say.
fn separators(node: &Node<String>, style: &Style) -> Vec<(String, Rect)> {
    let half = (style.separator.width + style.separator.extra_interact_width) * 0.5;
    let band = |rect: Rect, across_x: bool, fraction: f32| {
        if across_x {
            let x = rect.min.x + rect.width() * fraction;
            Rect::from_x_y_ranges(x - half..=x + half, rect.y_range())
        } else {
            let y = rect.min.y + rect.height() * fraction;
            Rect::from_x_y_ranges(rect.x_range(), y - half..=y + half)
        }
    };
    match node {
        Node::Linear(linear) => (0..linear.len().saturating_sub(1))
            .map(|i| {
                let horizontal = linear.dir == LinearDir::Horizontal;
                (
                    format!(
                        "after child {i} of the {}",
                        if horizontal { "row" } else { "column" }
                    ),
                    band(linear.rect, horizontal, linear.boundary(i)),
                )
            })
            .collect(),
        Node::Grid(grid) => (0..grid.columns().saturating_sub(1))
            .map(|i| {
                (
                    format!("grid line after column {i}"),
                    band(grid.rect, true, grid.col_boundary(i)),
                )
            })
            .chain((0..grid.rows().saturating_sub(1)).map(|i| {
                (
                    format!("grid line after row {i}"),
                    band(grid.rect, false, grid.row_boundary(i)),
                )
            }))
            .collect(),
        _ => Vec::new(),
    }
}

fn node_text(node: &Node<String>) -> String {
    match node {
        Node::Leaf(leaf) => {
            let mut text = format!("leaf, {} tabs, active {:?}", leaf.tabs.len(), leaf.active);
            if leaf.collapsed {
                text += ", collapsed";
            }
            if leaf.tab_bar_hidden {
                text += ", tab bar hidden";
            }
            text
        }
        Node::Linear(linear) => {
            format!("linear {}, {} children", dir_text(linear.dir), linear.len())
        }
        Node::Grid(grid) => format!(
            "grid, {} columns x {} rows, {:?}",
            grid.columns(),
            grid.rows(),
            grid.layout
        ),
        Node::Empty => "empty".to_owned(),
        _ => "another kind of node".to_owned(),
    }
}

fn parent_text(tree: &Tree<String>, node: NodeIndex) -> Option<String> {
    let parent = tree.parent(node)?;
    match &tree[parent] {
        Node::Linear(linear) => {
            let i = linear.position(node)?;
            Some(format!(
                "Parent: {parent:?}, linear {}, child {i} of {}, share {:.3}",
                dir_text(linear.dir),
                linear.len(),
                linear.fraction(i)
            ))
        }
        Node::Grid(grid) => {
            let i = grid.position(node)?;
            let (column, row) = grid.cell(i);
            Some(format!(
                "Parent: {parent:?}, grid {} x {}, cell column {column} row {row}, column share \
                 {:.3}, row share {:.3}",
                grid.columns(),
                grid.rows(),
                grid.col_fraction(column),
                grid.row_fraction(row)
            ))
        }
        _ => None,
    }
}

fn dir_text(dir: LinearDir) -> &'static str {
    match dir {
        LinearDir::Horizontal => "horizontal (a row)",
        LinearDir::Vertical => "vertical (a column)",
    }
}

fn tab_style_text(style: &TabInteractionStyle) -> String {
    format!(
        "bg_fill {}, text_color {}, outline_color {}, corner_radius {}",
        color(style.bg_fill),
        color(style.text_color),
        color(style.outline_color),
        radius(style.corner_radius)
    )
}

fn rect_text(rect: Rect) -> String {
    format!(
        "({:.1}, {:.1}) to ({:.1}, {:.1}), {:.1} x {:.1}",
        rect.min.x,
        rect.min.y,
        rect.max.x,
        rect.max.y,
        rect.width(),
        rect.height()
    )
}

fn color(color: Color32) -> String {
    let [r, g, b, a] = color.to_srgba_unmultiplied();
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

fn stroke(stroke: Stroke) -> String {
    format!("{} {}", stroke.width, color(stroke.color))
}

fn margin(margin: Margin) -> String {
    format!(
        "[{} {} {} {}]",
        margin.left, margin.right, margin.top, margin.bottom
    )
}

fn radius(radius: CornerRadius) -> String {
    if radius.nw == radius.ne && radius.ne == radius.sw && radius.sw == radius.se {
        radius.nw.to_string()
    } else {
        format!("[{} {} {} {}]", radius.nw, radius.ne, radius.sw, radius.se)
    }
}
