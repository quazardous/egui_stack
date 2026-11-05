//! Runs the demo: in a native window, or in the browser when built for `wasm32` (see
//! `index.html`).

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // no console window on Windows in release

use egui_stack_demo::DemoApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui_stack demo",
        options,
        Box::new(|_| Ok(Box::<DemoApp>::default())),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;
    use eframe::web_sys;

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("a document");
        let canvas = document
            .get_element_by_id("demo_canvas")
            .expect("a canvas with the id `demo_canvas`")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("`demo_canvas` is a canvas");
        let started = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|_| Ok(Box::<DemoApp>::default())),
            )
            .await;
        if let Some(loading) = document.get_element_by_id("loading") {
            match started {
                Ok(()) => loading.remove(),
                Err(error) => {
                    loading.set_inner_html("The demo failed to start; see the console.");
                    web_sys::console::error_1(&error);
                }
            }
        }
    });
}
