//! The egui_stack demo: a dock to play with, natively or in the browser.
//!
//! Drag a tab over a leaf: the band along each edge splits the leaf on that side, the rest
//! adds the tab to the leaf's tabs, and outside the dock the tab opens in a floating window.
//! The toolbar switches the overlay, sets the width of the edge bands, arranges the focused
//! leaf's container as a grid, a row or a column, opens floating windows, and saves and
//! restores the layout. Its Inspect button picks a part of the dock and says what it is (see
//! [`picker`]), and its Focus border toggle shows a custom [`DockPainter`] outlining the
//! focused leaf.
//!
//! Its second page, Dock space, shows a [`DockSpace`] of three docks: one in a left side panel,
//! one in a right side panel and one in the centre, each drawn by its own [`DockArea`], with
//! tabs dragged between them into the same zones.
//!
//! On both pages, the dock is a [`DockSpace`] (the first page's has one dock) and a tab or a
//! group moved out of every dock (or the toolbar's Float group or New float) opens in a
//! floating window, drawn by a [`FloatsArea`] over the window or, natively, in a window of its
//! own: the handle at the right end of its top tab bar moves it.

use egui::{
    Color32, Id, Panel, Rect, RichText, Slider, Stroke, Ui, UiBuilder, WidgetText, pos2, vec2,
};
use egui_stack::layout::{Content, FloatHost, Layout, Zone};
use egui_stack::{
    AllowedSplits, DockArea, DockPainter, DockRules, DockSpace, DockState, EguiViewports,
    FloatsArea, GridLayout, GroupHandles, LinearDir, MoveFrom, NodeIndex, NodePath, OverlayType,
    PartState, Style, StylePainter, TabAddAlign, TabBodyPaint, TabViewer, Tree,
};

pub mod picker;

use picker::Picker;

/// The demo's state: the dock, the tabs' contents and the toolbar's settings.
pub struct DemoApp {
    /// The page shown: one dock, or a space of three docks.
    pub page: Page,
    /// The layout of the first page: one dock, `main`, and the floats' docks.
    pub dock: DockSpace<String>,
    /// The floating windows of the first page, over the window or native ([`FloatsArea`]).
    /// Its zones are not drawn: the page's dock fills the window.
    pub dock_floats: Layout,
    /// The layout of the second page: three docks and the floats' docks.
    pub space: DockSpace<String>,
    /// The floating windows of the second page, over the window or native ([`FloatsArea`]).
    /// Its zones are not drawn: the page's docks are in panels.
    pub floats: Layout,
    /// What the tabs show.
    pub tabs: Tabs,
    /// The overlay shown while a tab is dragged.
    pub overlay_type: OverlayType,
    /// The share of a leaf the band along each of its edges covers, with the zones overlay.
    pub zone_edge_coverage: f32,
    /// The inspection picker and its panel.
    pub picker: Picker,
    /// Whether the focused leaf's body is outlined, by [`FocusBorder`].
    pub focus_border: bool,
    /// The handles that move a whole tab group: all of them by default.
    pub group_handles: GroupHandles,
    /// Where the add tab button sits: right after the last tab by default.
    pub add_tab_align: TabAddAlign,
    /// Whether tab bars show an Eject group button (`DockArea::show_leaf_eject_buttons`).
    pub eject_buttons: bool,
    /// Whether a float moves from its empty space and by Alt+drag anywhere
    /// (`FloatsArea::move_from`), not only by its move handle.
    pub move_floats_anywhere: bool,
    saved: Option<String>,
    status: String,
    opened: usize,
}

/// The demo's pages.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// One dock, filling the window.
    Dock,
    /// A [`DockSpace`]: a dock on each side and one in the centre.
    Space,
}

/// The tabs' contents.
pub struct Tabs {
    notes: String,
    code: String,
    add_request: Option<NodePath>,
    /// The tab buttons' rects of the frame, which the picker hit-tests.
    tab_rects: Vec<(String, Rect)>,
    /// Whether a dragged tab's ghost previews its content (`TabViewer::ghost_body`).
    pub ghost_preview: bool,
}

impl Default for DemoApp {
    fn default() -> Self {
        Self {
            page: Page::Dock,
            dock: initial_dock(),
            dock_floats: initial_dock_floats(),
            space: initial_space(),
            floats: initial_floats(),
            tabs: Tabs {
                notes:
                    "Things to try:\n\n- drag this tab to another leaf's edge\n- drag it out of \
                        the dock\n- arrange a row as a grid\n- move a whole group by the dotted grip at the \
                        end of its tab bar, or drag a tab with Shift held"
                        .to_owned(),
                code: "fn main() {\n    println!(\"Hello, docking!\");\n}\n".to_owned(),
                add_request: None,
                tab_rects: Vec::new(),
                ghost_preview: false,
            },
            overlay_type: OverlayType::Zones,
            zone_edge_coverage: Style::default().overlay.feel.zone_edge_coverage,
            picker: Picker::default(),
            focus_border: false,
            group_handles: GroupHandles::ALL,
            add_tab_align: TabAddAlign::Left,
            eject_buttons: false,
            move_floats_anywhere: false,
            saved: None,
            status: String::new(),
            opened: 0,
        }
    }
}

/// The first page's space, of one dock, `main`: a row of three leaves, the guide, an editor
/// with a console and notes, and a preview. It has an id of its own, apart from the second
/// page's space.
pub fn initial_dock() -> DockSpace<String> {
    let mut tree = Tree::new(vec!["Guide".to_owned()]);
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
    let mut space = DockSpace::new().with_id(Id::unique("egui_stack_demo_one_dock"));
    space.add_dock_tree("main", DockRules::default(), tree);
    space
}

/// The floats of the first page: none at first.
fn initial_dock_floats() -> Layout {
    Layout::new(Zone::leaf("main", Content::Dock("main".into())))
}

/// The floats of the second page: none at first.
fn initial_floats() -> Layout {
    Layout::new(Zone::leaf("centre", Content::Dock("centre".into())))
}

/// Where the toolbar's Float group opens a float.
const FLOAT_RECT: Rect = Rect::from_min_max(pos2(240.0, 160.0), pos2(640.0, 440.0));

/// The space of the second page: files and an outline on the left, which splits only into
/// rows; the guide and an editor in the centre; a properties panel on the right, kept as an
/// empty target once its tabs leave.
pub fn initial_space() -> DockSpace<String> {
    let mut space = DockSpace::new();
    space.add_dock(
        "left",
        DockRules::default().splits(AllowedSplits::TopBottomOnly),
        vec!["Files".to_owned(), "Outline".to_owned()],
    );
    space.add_dock(
        "centre",
        DockRules::default(),
        vec!["Space guide".to_owned(), "Editor".to_owned()],
    );
    space.add_dock(
        "right",
        DockRules::default().keep_when_empty(true),
        vec!["Properties".to_owned()],
    );
    space
}

impl DemoApp {
    /// Shows the toolbar and the page in `ui`.
    pub fn show(&mut self, ui: &mut Ui) {
        self.picker.keys(ui);
        Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));

        let mut style = Style::from_egui(ui.style());
        style.overlay.overlay_type = self.overlay_type;
        style.overlay.feel.zone_edge_coverage = self.zone_edge_coverage;
        style.buttons.add_tab_align = self.add_tab_align;
        let painter: &(dyn DockPainter + Sync) = if self.focus_border {
            &FocusBorder
        } else {
            &StylePainter
        };
        self.tabs.tab_rects.clear();
        if self.page == Page::Space {
            self.show_space(ui, &style, painter);
            self.take_add_request();
            return;
        }
        self.picker.panel(ui);

        let dock_rect = ui.available_rect_before_wrap();
        DockArea::dock(&mut self.dock, "main")
            .style(style.clone())
            .painter(painter)
            .group_handles(self.group_handles)
            .show_add_buttons(true)
            .show_leaf_collapse_buttons(true)
            .show_leaf_eject_buttons(self.eject_buttons)
            .show_inside(ui, &mut self.tabs);
        // The floats, after the dock: their top tab bar ends with the window's move handle.
        let move_from = self.move_from();
        FloatsArea::new(&mut self.dock_floats, &mut self.dock)
            .host(&mut EguiViewports)
            .style(style.clone())
            .painter(painter)
            .group_handles(self.group_handles)
            // As in the dock it came from: a floating group keeps its add button.
            .show_add_buttons(true)
            .show_leaf_eject_buttons(self.eject_buttons)
            .move_from(move_from)
            .show(ui.ctx(), &mut self.tabs);
        self.dock.take_float_events();
        self.picker.after_dock(
            ui,
            dock_rect,
            self.dock.state(),
            &self.tabs.tab_rects,
            &style,
        );

        self.take_add_request();
    }

    /// Where a float moves from, besides its move handle ([`FloatsArea::move_from`]).
    fn move_from(&self) -> MoveFrom {
        if self.move_floats_anywhere {
            MoveFrom::EMPTY_SPACE | MoveFrom::modifier(egui::Modifiers::ALT)
        } else {
            MoveFrom::BAR
        }
    }

    /// The second page: each dock of the space in its own place, drawn by its own call, the
    /// side ones in resizable side panels.
    fn show_space(&mut self, ui: &mut Ui, style: &Style, painter: &(dyn DockPainter + Sync)) {
        for (name, panel) in [
            ("left", Panel::left("space-left")),
            ("right", Panel::right("space-right")),
        ] {
            panel
                .resizable(true)
                .default_size(240.0)
                .min_size(120.0)
                .show(ui, |ui| {
                    DockArea::dock(&mut self.space, name)
                        .style(style.clone())
                        .painter(painter)
                        .group_handles(self.group_handles)
                        .show_leaf_collapse_buttons(true)
                        .show_leaf_eject_buttons(self.eject_buttons)
                        .show_inside(ui, &mut self.tabs);
                });
        }
        DockArea::dock(&mut self.space, "centre")
            .style(style.clone())
            .painter(painter)
            .group_handles(self.group_handles)
            .show_add_buttons(true)
            .show_leaf_collapse_buttons(true)
            .show_leaf_eject_buttons(self.eject_buttons)
            .show_inside(ui, &mut self.tabs);
        // The floats, after the docks: their top tab bar ends with the window's move handle.
        let move_from = self.move_from();
        FloatsArea::new(&mut self.floats, &mut self.space)
            .host(&mut EguiViewports)
            .style(style.clone())
            .painter(painter)
            .group_handles(self.group_handles)
            // As in the dock it came from: a floating group keeps its add button.
            .show_add_buttons(true)
            .show_leaf_eject_buttons(self.eject_buttons)
            .move_from(move_from)
            .show(ui.ctx(), &mut self.tabs);
        self.space.take_float_events();
    }

    /// The space of the page shown.
    fn page_space(&mut self) -> &mut DockSpace<String> {
        match self.page {
            Page::Dock => &mut self.dock,
            Page::Space => &mut self.space,
        }
    }

    /// Moves the focused group of the page shown into a float of its own.
    fn float_focused_group(&mut self) {
        let space = self.page_space();
        if let Some(path) = space.state().focused_leaf() {
            space.float_out_group(path, FLOAT_RECT, |tab| Id::unique(tab.as_str()));
        }
    }

    /// Opens a new tab in a float of its own, on the page shown.
    fn new_float(&mut self) {
        self.opened += 1;
        let tab = format!("Tab {}", self.opened);
        let space = self.page_space();
        let dock = space.dock_names().next().map(str::to_owned);
        if let Some(tree) = dock.and_then(|dock| space.dock_mut(&dock)) {
            tree.push_to_first_leaf(tab.clone());
        }
        if let Some(path) = space.state().find_tab(&tab) {
            space.float_out(path, FLOAT_RECT, |tab| Id::unique(tab.as_str()));
        }
    }

    /// A leaf's add button (`on_add`) asks for a tab here.
    fn take_add_request(&mut self) {
        if let Some(path) = self.tabs.add_request.take() {
            self.opened += 1;
            let tab = format!("Tab {}", self.opened);
            let state = self.state_mut();
            state.set_focused_node_and_surface(path);
            state.push_to_focused_leaf(tab);
        }
    }

    /// The layout of the page shown.
    fn state(&self) -> &DockState<String> {
        match self.page {
            Page::Dock => self.dock.state(),
            Page::Space => self.space.state(),
        }
    }

    /// The layout of the page shown, mutably.
    fn state_mut(&mut self) -> &mut DockState<String> {
        match self.page {
            Page::Dock => self.dock.state_mut(),
            Page::Space => self.space.state_mut(),
        }
    }

    fn toolbar(&mut self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut self.page, Page::Dock, "One dock")
                .on_hover_text("One dock filling the window");
            ui.selectable_value(&mut self.page, Page::Space, "Dock space")
                .on_hover_text("Three docks, left, centre and right, sharing one drag");

            ui.separator();
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
                    let tree = &mut self.state_mut()[path.surface];
                    match arrangement {
                        Arrangement::Grid => tree.arrange_as_grid(path.node, GridLayout::Auto),
                        Arrangement::Linear(dir) => tree.arrange_as_linear(path.node, dir),
                    };
                }
            }

            ui.separator();
            ui.label("Floats:");
            // Native windows where the platform has them: not in the browser.
            if cfg!(not(target_arch = "wasm32")) {
                let space = self.page_space();
                let mut host = space.float_host();
                ui.selectable_value(&mut host, FloatHost::InWindow, "In the window")
                    .on_hover_text("Floating windows drawn over this window");
                ui.selectable_value(&mut host, FloatHost::Native, "Native")
                    .on_hover_text("Floating windows of their own, on the desktop");
                if host != space.float_host() {
                    space.set_float_host(host);
                }
            }
            let focused = self.state().focused_leaf().is_some();
            if ui
                .add_enabled(focused, egui::Button::new("Float group"))
                .on_hover_text(
                    "Move the focused group into a floating window, moved by the handle at the \
                     end of its tab bar",
                )
                .on_disabled_hover_text("Click a tab first")
                .clicked()
            {
                self.float_focused_group();
            }
            if ui
                .button("New float")
                .on_hover_text("Open a new tab in a floating window")
                .clicked()
            {
                self.new_float();
            }

            ui.separator();
            ui.toggle_value(&mut self.focus_border, "Focus border")
                .on_hover_text("Outline the focused leaf, with a custom DockPainter");

            ui.menu_button("Group handles", |ui| {
                for (flag, label, hint) in [
                    (
                        GroupHandles::GRIP,
                        "Grip",
                        "The dotted grip at the end of each tab bar",
                    ),
                    (
                        GroupHandles::EMPTY_SPACE,
                        "Empty space",
                        "The tab bar right of its tabs",
                    ),
                    (
                        GroupHandles::MODIFIER_DRAG,
                        "Shift + drag",
                        "A tab dragged with Shift held",
                    ),
                    (
                        GroupHandles::MENU,
                        "Menu",
                        "Move group in the tab's context menu",
                    ),
                    (
                        GroupHandles::MOVE_WINDOW,
                        "Window move handle",
                        "At the end of a floating window's top tab bar, it moves the window",
                    ),
                ] {
                    let mut on = self.group_handles.contains(flag);
                    if ui.checkbox(&mut on, label).on_hover_text(hint).changed() {
                        self.group_handles = self.group_handles.with(flag, on);
                    }
                }
            })
            .response
            .on_hover_text("What moves a whole tab group");

            ui.toggle_value(&mut self.tabs.ghost_preview, "Preview in the ghost")
                .on_hover_text("Preview tab content in the ghost of a dragged tab or group");

            ui.toggle_value(&mut self.move_floats_anywhere, "Move floats from anywhere")
                .on_hover_text(
                    "A floating window moves by a drag on its empty space, or by Alt+drag \
                     anywhere in it, not only by its move handle",
                );

            ui.toggle_value(&mut self.eject_buttons, "Eject group buttons")
                .on_hover_text("A button on each tab bar moves its group into a new window");

            let mut at_end = self.add_tab_align == TabAddAlign::Right;
            if ui
                .toggle_value(&mut at_end, "+ at the end")
                .on_hover_text(
                    "The add tab button at the end of the tab bar, not after the last tab",
                )
                .changed()
            {
                self.add_tab_align = if at_end {
                    TabAddAlign::Right
                } else {
                    TabAddAlign::Left
                };
            }

            if self.page == Page::Dock {
                ui.separator();
                self.picker.button(ui);
            }

            ui.separator();
            if ui.button("Save layout").clicked() {
                let json = match self.page {
                    Page::Dock => serde_json::to_string(&(&self.dock, &self.dock_floats)),
                    Page::Space => serde_json::to_string(&(&self.space, &self.floats)),
                };
                match json {
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
                let restored = match self.page {
                    Page::Dock => serde_json::from_str(json).map(|(dock, floats)| {
                        self.dock = dock;
                        self.dock_floats = floats;
                    }),
                    Page::Space => serde_json::from_str(json).map(|(space, floats)| {
                        self.space = space;
                        self.floats = floats;
                    }),
                };
                self.status = match restored {
                    Ok(()) => "Restored".to_owned(),
                    Err(error) => format!("Not restored: {error}"),
                };
            }
            if ui.button("Reset").clicked() {
                match self.page {
                    Page::Dock => {
                        self.dock = initial_dock();
                        self.dock_floats = initial_dock_floats();
                    }
                    Page::Space => {
                        self.space = initial_space();
                        self.floats = initial_floats();
                    }
                }
                self.status = "Back to the first layout".to_owned();
            }
            if !self.status.is_empty() {
                ui.weak(&self.status);
            }
        });
    }

    /// The container (a row, a column or a grid) holding the focused leaf.
    fn focused_container(&self) -> Option<NodePath> {
        let state = self.state();
        let leaf = state.focused_leaf()?;
        let node = state[leaf.surface].parent(leaf.node)?;
        Some(NodePath {
            surface: leaf.surface,
            node,
        })
    }
}

/// A painter outlining the focused leaf's body in the selection colour; the rest as the style
/// says. The style itself draws no border around the bodies.
pub struct FocusBorder;

impl DockPainter for FocusBorder {
    fn tab_body(&self, painter: &egui::Painter, body: &TabBodyPaint<'_>) {
        if body.state.contains(PartState::FOCUSED) {
            let stroke = painter.ctx().global_style().visuals.selection.stroke;
            painter.rect_stroke(
                body.rect,
                body.corner_radius,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
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
            "Space guide" => space_guide(ui),
            "Files" => {
                for file in ["src/", "  main.rs", "  lib.rs", "Cargo.toml", "README.md"] {
                    ui.monospace(file);
                }
            }
            "Outline" => {
                for item in ["fn main", "struct App", "impl App"] {
                    ui.monospace(item);
                }
            }
            "Properties" => {
                ui.label("Nothing selected.");
            }
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

    fn ghost_size(&mut self, _tab: &String, default: egui::Vec2) -> egui::Vec2 {
        if self.ghost_preview {
            // As large as the ghost goes: room for the preview.
            vec2(320.0, 200.0)
        } else {
            default
        }
    }

    fn ghost_body(&mut self, ui: &mut Ui, tab: &String, rect: Rect) -> bool {
        if !self.ghost_preview {
            return false;
        }
        ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
        ui.scope_builder(UiBuilder::new().max_rect(rect.shrink(6.0)), |ui| {
            let mut tab = tab.clone();
            self.ui(ui, &mut tab);
        });
        true
    }
}

fn guide(ui: &mut Ui) {
    ui.heading("egui_stack");
    ui.label("Docking for egui. Things to try:");
    for line in [
        "Drag a tab by its title onto another leaf: near an edge, the leaf splits on that side; \
         in the middle, the tab joins its tabs.",
        "Drag a tab out of the dock: it opens in a floating window, which takes drops too. \
         The handle at the right end of its top tab bar moves the window; the toolbar's Floats \
         choice opens the next ones in native windows of their own.",
        "Drag the separators between leaves to resize them.",
        "Click a leaf, then Arrange: its row becomes a grid, a column, or a row again.",
        "Switch the overlay to the cross (egui_dock's former default) or highlighted areas.",
        "Save the layout, change it, then restore it.",
        "Focus border: a custom painter outlines the focused leaf, which no style field does.",
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

fn space_guide(ui: &mut Ui) {
    ui.heading("A dock space");
    ui.label("Three docks, each drawn by its own DockArea, sharing one drag:");
    for line in [
        "Drag a tab from a side dock to the centre, or back: the edge bands split the leaf \
         under the pointer, its middle adds the tab, a tab of its bar takes the tab before it.",
        "The left dock splits only into rows: its left and right bands are not offered.",
        "Empty the right dock: it stays, as a target for the next tab.",
        "Drag a tab out of every dock, or a whole group by its grip: it opens in a floating \
         window, which takes drops too. Float group in the toolbar does it for the focused \
         group, and so does a grip's menu, Move group > New window.",
        "Move a floating window by the handle at the right end of its top tab bar; the \
         toolbar's Floats choice opens the next ones in native windows of their own.",
        "Resize the side panels: each dock fills the place the application gives it.",
    ] {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("•").strong());
            ui.label(line);
        });
    }
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
