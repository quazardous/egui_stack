---
egui_stack: patch
---

The demo's first page, One dock, draws its floating windows with `FloatsArea`, as the Dock space page does: a tab dragged out of its dock opens a float with the window's move handle, over the window or natively, where it opened a legacy `egui::Window` before. The toolbar's New window is now New float, on both pages, and the Floats choice (in the window or native) is offered on both, natively only outside the browser. `DockState::add_window`'s documentation points to floats.
