//! The egui_stack demo: a dock to play with, natively or in the browser.
//!
//! Drag a tab over a leaf: the band along each edge splits the leaf on that side, the rest
//! adds the tab to the leaf's tabs, and outside the dock the tab opens in a window. The
//! toolbar switches the overlay, sets the width of the edge bands, arranges the focused
//! leaf's container as a grid, a row or a column, opens windows, and saves and restores the
//! layout. Its Inspect button picks a part of the dock and says what it is (see [`picker`]).

use egui::{Color32, Id, Panel, Rect, RichText, Slider, Stroke, Ui, WidgetText, pos2, vec2};
use egui_stack::{
    DockArea, DockState, GridLayout, LinearDir, NodeIndex, NodePath, OverlayType, Style, TabViewer,
};

pub mod picker;

use picker::Picker;

/// The demo's state: the dock, the tabs' contents and the toolbar's settings.
pub struct DemoApp {
    /// The layout: the main surface and the windows.
    pub dock_state: DockState<String>,
    /// What the tabs show.
    pub tabs: Tabs,
    /// The overlay shown while a tab is dragged.
    pub overlay_type: OverlayType,
    /// The share of a leaf the band along each of its edges covers, with the zones overlay.
    pub zone_edge_coverage: f32,
    /// The inspection picker and its panel.
    pub picker: Picker,
    saved: Option<String>,
    status: String,
    opened: usize,
}

/// The tabs' contents.
pub struct Tabs {
    notes: String,
    code: String,
    add_request: Option<NodePath>,
    /// The tab buttons' rects of the frame, which the picker hit-tests.
    tab_rects: Vec<(String, Rect)>,
}

impl Default for DemoApp {
    fn default() -> Self {
        Self {
            dock_state: initial_layout(),
            tabs: Tabs {
                notes:
                    "Things to try:\n\n- drag this tab to another leaf's edge\n- drag it out of \
                        the dock\n- arrange a row as a grid"
                        .to_owned(),
                code: "fn main() {\n    println!(\"Hello, docking!\");\n}\n".to_owned(),
                add_request: None,
                tab_rects: Vec::new(),
            },
            overlay_type: OverlayType::Zones,
            zone_edge_coverage: Style::default().overlay.feel.zone_edge_coverage,
            picker: Picker::default(),
            saved: None,
            status: String::new(),
            opened: 0,
        }
    }
}

/// A row of three leaves: the guide, an editor with a console and notes, and a preview.
fn initial_layout() -> DockState<String> {
    let mut dock_state = DockState::new(vec!["Guide".to_owned()]);
    let tree = dock_state.main_surface_mut();
    let [_, middle] = tree.split_right(
        NodeIndex::root(),
        0.3,
        vec![
            "Editor".to_owned(),
            "Console".to_owned(),
            "Notes".to_owned(),
        ],
    );
    tree.split_right(middle, 0.55, vec!["Preview".to_owned()]);
    dock_state
}

impl DemoApp {
    /// Shows the toolbar and the dock in `ui`.
    pub fn show(&mut self, ui: &mut Ui) {
        self.picker.keys(ui);
        Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        self.picker.panel(ui);

        let mut style = Style::from_egui(ui.style());
        style.overlay.overlay_type = self.overlay_type;
        style.overlay.feel.zone_edge_coverage = self.zone_edge_coverage;
        let dock_rect = ui.available_rect_before_wrap();
        self.tabs.tab_rects.clear();
        DockArea::new(&mut self.dock_state)
            .style(style.clone())
            .show_add_buttons(true)
            .show_leaf_collapse_buttons(true)
            .show_inside(ui, &mut self.tabs);
        self.picker.after_dock(
            ui,
            dock_rect,
            &self.dock_state,
            &self.tabs.tab_rects,
            &style,
        );

        // A leaf's add button (`on_add`) asks for a tab here.
        if let Some(path) = self.tabs.add_request.take() {
            self.opened += 1;
            let tab = format!("Tab {}", self.opened);
            self.dock_state.set_focused_node_and_surface(path);
            self.dock_state.push_to_focused_leaf(tab);
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label("Overlay:");
            ui.selectable_value(&mut self.overlay_type, OverlayType::Zones, "Drop zones")
                .on_hover_text("The region a dropped tab would take, on the leaf under it");
            ui.selectable_value(&mut self.overlay_type, OverlayType::Widgets, "Cross")
                .on_hover_text("Buttons to drop a tab on, egui_dock's former default");
            ui.selectable_value(
                &mut self.overlay_type,
                OverlayType::HighlightedAreas,
                "Highlighted areas",
            );
            ui.add_enabled(
                self.overlay_type == OverlayType::Zones,
                Slider::new(&mut self.zone_edge_coverage, 0.05..=0.5)
                    .text("Edge band")
                    .custom_formatter(|value, _| format!("{:.0}%", value * 100.0))
                    .custom_parser(|text| {
                        text.trim_end_matches('%')
                            .parse::<f64>()
                            .ok()
                            .map(|v| v / 100.0)
                    }),
            )
            .on_hover_text("How much of a leaf the band along each edge covers");

            ui.separator();
            let container = self.focused_container();
            ui.label("Arrange:");
            for (label, arrangement) in [
                ("Grid", Arrangement::Grid),
                ("Row", Arrangement::Linear(LinearDir::Horizontal)),
                ("Column", Arrangement::Linear(LinearDir::Vertical)),
            ] {
                let button = ui
                    .add_enabled(container.is_some(), egui::Button::new(label))
                    .on_hover_text("Arrange the focused leaf and its siblings")
                    .on_disabled_hover_text("Focus a leaf that has siblings first");
                if button.clicked()
                    && let Some(path) = container
                {
                    let tree = &mut self.dock_state[path.surface];
                    match arrangement {
                        Arrangement::Grid => tree.arrange_as_grid(path.node, GridLayout::Auto),
                        Arrangement::Linear(dir) => tree.arrange_as_linear(path.node, dir),
                    };
                }
            }

            ui.separator();
            if ui.button("New window").clicked() {
                self.opened += 1;
                let tab = format!("Tab {}", self.opened);
                self.dock_state.add_window(vec![tab]);
            }

            ui.separator();
            self.picker.button(ui);

            ui.separator();
            if ui.button("Save layout").clicked() {
                match serde_json::to_string(&self.dock_state) {
                    Ok(json) => {
                        self.status = format!("Saved ({} bytes of JSON)", json.len());
                        self.saved = Some(json);
                    }
                    Err(error) => self.status = format!("Not saved: {error}"),
                }
            }
            if ui
                .add_enabled(self.saved.is_some(), egui::Button::new("Restore"))
                .clicked()
                && let Some(json) = &self.saved
            {
                match serde_json::from_str(json) {
                    Ok(dock_state) => {
                        self.dock_state = dock_state;
                        self.status = "Restored".to_owned();
                    }
                    Err(error) => self.status = format!("Not restored: {error}"),
                }
            }
            if ui.button("Reset").clicked() {
                self.dock_state = initial_layout();
                self.status = "Back to the first layout".to_owned();
            }
            if !self.status.is_empty() {
                ui.weak(&self.status);
            }
        });
    }

    /// The container (a row, a column or a grid) holding the focused leaf.
    fn focused_container(&self) -> Option<NodePath> {
        let leaf = self.dock_state.focused_leaf()?;
        let node = self.dock_state[leaf.surface].parent(leaf.node)?;
        Some(NodePath {
            surface: leaf.surface,
            node,
        })
    }
}

enum Arrangement {
    Grid,
    Linear(LinearDir),
}

impl eframe::App for DemoApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

impl TabViewer for Tabs {
    type Tab = String;

    fn id(&mut self, tab: &mut String) -> Id {
        Id::unique(tab.as_str())
    }

    fn title(&mut self, tab: &mut String) -> WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut String) {
        match tab.as_str() {
            "Guide" => guide(ui),
            "Editor" => {
                let label = ui.weak("main.rs");
                ui.add(
                    egui::TextEdit::multiline(&mut self.code)
                        .code_editor()
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(label.id);
            }
            "Console" => {
                for line in [
                    "[info] dock ready",
                    "[info] 5 tabs in 3 leaves",
                    "[hint] drop a tab on an edge band to split",
                ] {
                    ui.monospace(line);
                }
            }
            "Notes" => {
                let label = ui.weak("Notes");
                ui.add(egui::TextEdit::multiline(&mut self.notes).desired_width(f32::INFINITY))
                    .labelled_by(label.id);
            }
            "Preview" => preview(ui),
            _ => {
                ui.label(format!("{tab}: a tab opened at run time."));
            }
        }
    }

    fn on_tab_button(&mut self, tab: &mut String, response: &egui::Response) {
        self.tab_rects.push((tab.clone(), response.rect));
    }

    fn on_add(&mut self, path: NodePath) {
        self.add_request = Some(path);
    }
}

fn guide(ui: &mut Ui) {
    ui.heading("egui_stack");
    ui.label("Docking for egui. Things to try:");
    for line in [
        "Drag a tab by its title onto another leaf: near an edge, the leaf splits on that side; \
         in the middle, the tab joins its tabs.",
        "Drag a tab out of the dock: it opens in a floating window, which takes drops too.",
        "Drag the separators between leaves to resize them.",
        "Click a leaf, then Arrange: its row becomes a grid, a column, or a row again.",
        "Switch the overlay to the cross (egui_dock's former default) or highlighted areas.",
        "Save the layout, change it, then restore it.",
        "Inspect (Ctrl+Shift+F12), then point at a part of the dock: a panel says what it is \
         and which style fields drew it.",
    ] {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("•").strong());
            ui.label(line);
        });
    }
    ui.add_space(8.0);
    ui.hyperlink_to(
        "Source on GitHub",
        "https://github.com/quazardous/egui_stack",
    );
}

/// A few shapes, to give the preview something to show.
fn preview(ui: &mut Ui) {
    let size = ui.available_size().max(vec2(40.0, 40.0));
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = rect.width().min(rect.height()) * 0.3;
    painter.circle_filled(center, radius, Color32::from_rgb(70, 110, 170));
    painter.circle_stroke(
        center,
        radius * 1.3,
        Stroke::new(2.0, Color32::from_gray(150)),
    );
    let bar = radius * 0.4;
    painter.rect_filled(
        egui::Rect::from_min_max(
            pos2(center.x - bar, center.y - bar * 0.3),
            pos2(center.x + bar, center.y + bar * 0.3),
        ),
        2.0,
        Color32::from_rgb(230, 180, 60),
    );
}
