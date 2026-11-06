# egui_stack demo

A dock to play with: tabs to drag between leaves, into the edge bands that split them and out
into floating windows; a toolbar to switch the drop overlay (drop zones, the cross, highlighted
areas), set the width of the edge bands, arrange the focused leaf's row, column or grid, open
windows, save and restore the layout, outline the focused leaf with a custom `DockPainter`, and
inspect a part of the dock (Ctrl+Shift+F12): a side panel says what it is and which `Style`
fields drew it, with a Copy button for bug reports.

It is a separate crate, not published, so that its dependencies (the web ones included) are not
the library's.

- Natively: `cargo run -p egui_stack_demo`.
- In the browser: `sh demo/build-web.sh` builds it into `demo/dist`, which any static file
  server can serve, for example `python3 -m http.server -d demo/dist`. It needs the
  `wasm32-unknown-unknown` target and the `wasm-bindgen` CLI at the version `Cargo.toml` pins
  (`cargo install wasm-bindgen-cli --version <it>`). The [online demo](https://quazardous.github.io/egui_stack/)
  is built the same way on each push to the main branch (`.github/workflows/pages.yml`).
- The README's GIF, `images/demo.gif`, is recorded headless from the demo:
  `cargo test -p egui_stack_demo --test record_gif -- --ignored` (it renders with wgpu, so it
  needs a GPU or a software Vulkan driver).
