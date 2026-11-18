#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

//! Native floating docks: drag a tab out of the window (or use "Eject" in its menu) and it
//! opens in a native window of its own, drawn with egui's immediate viewports. Close that
//! window and its tabs go back where they came from; the checkbox in the status bar sends them
//! to the right dock instead. The status bar lists the windows' events.
//!
//! Floats over the main window instead: start with `--in-window`.

use eframe::NativeOptions;
use egui::{Ui, WidgetText};
use egui_stack::layout::{
    Content, Dir, FloatHost, Layout, LayoutArea, Size, Zone, ZoneName, ZoneViewer,
};
use egui_stack::{
    ClosePolicy, DockArea, DockRules, DockSpace, Docked, EguiViewports, FloatClose, FloatEvent,
    TabViewer,
};

fn main() -> eframe::Result<()> {
    let host = if std::env::args().any(|arg| arg == "--in-window") {
        FloatHost::InWindow
    } else {
        FloatHost::Native
    };
    eframe::run_native(
        "Native floats",
        NativeOptions::default(),
        Box::new(move |_cc| Ok(Box::new(App::new(host)))),
    )
}

struct App {
    docked: Docked<String>,
    tabs: Tabs,
    log: Vec<String>,
}

impl App {
    fn new(host: FloatHost) -> Self {
        let mut space = DockSpace::new().with_float_host(host);
        space.add_dock(
            "centre",
            DockRules::default(),
            ["Editor", "Preview", "Notes"].map(str::to_owned).to_vec(),
        );
        space.add_dock(
            "right",
            DockRules::default(),
            ["Outline", "Search"].map(str::to_owned).to_vec(),
        );
        let layout = Layout::new(Zone::split(
            "root",
            Dir::Vertical,
            vec![
                Zone::split(
                    "main",
                    Dir::Horizontal,
                    vec![
                        Zone::leaf("centre", Content::Dock("centre".into()))
                            .with(|c| c.grow = true),
                        Zone::leaf("right", Content::Dock("right".into()))
                            .with(|c| c.default = Some(Size::Points(240.0))),
                    ],
                )
                .with(|c| c.grow = true),
                Zone::leaf("status", Content::App).with(|c| {
                    c.min = Some(Size::Points(72.0));
                    c.max = Some(Size::Points(72.0));
                }),
            ],
        ));
        Self {
            docked: Docked::new(layout, space),
            tabs: Tabs { to_right: false },
            log: Vec::new(),
        }
    }
}

struct Tabs {
    /// Closed windows send their tabs to the right dock.
    to_right: bool,
}

impl TabViewer for Tabs {
    type Tab = String;

    fn id(&mut self, tab: &mut String) -> egui::Id {
        egui::Id::unique(tab.as_str())
    }

    fn title(&mut self, tab: &mut String) -> WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut String) {
        ui.label(format!("{tab}: drag my tab out of the window."));
    }

    fn close_policy(&mut self, _close: &FloatClose) -> ClosePolicy {
        if self.to_right {
            ClosePolicy::Into("right".to_owned())
        } else {
            ClosePolicy::ReturnToOrigin
        }
    }
}

struct Viewer<'a> {
    space: &'a mut DockSpace<String>,
    tabs: &'a mut Tabs,
    log: &'a [String],
}

impl ZoneViewer for Viewer<'_> {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        match zone.0.as_str() {
            "status" => {
                ui.checkbox(
                    &mut self.tabs.to_right,
                    "A closed window sends its tabs to the right dock",
                );
                for line in self.log.iter().rev().take(2) {
                    ui.small(line);
                }
            }
            dock => DockArea::dock(self.space, dock).show_inside(ui, self.tabs),
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let laid = LayoutArea::new(&mut self.docked.layout).show_inside(
            ui,
            &mut Viewer {
                space: &mut self.docked.space,
                tabs: &mut self.tabs,
                log: &self.log,
            },
        );
        self.docked.space.bind(&laid);
        // The floats, after the docks: native windows through egui's immediate viewports.
        self.docked
            .floats()
            .host(&mut EguiViewports)
            .show(ui.ctx(), &mut self.tabs);
        for event in self.docked.space.take_float_events() {
            self.log.push(match event {
                FloatEvent::Detached { key, .. } => format!("{key} opened"),
                FloatEvent::Closed { key, moved, .. } => {
                    format!("{key} closed, {} tab(s) moved", moved.len())
                }
                FloatEvent::Moved { key, .. } => format!("{key} moved"),
                FloatEvent::Repaired { key, to, .. } => format!("{key} brought onto {to:?}"),
                _ => "?".to_owned(),
            });
        }
    }
}
