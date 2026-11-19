---
egui_stack: minor
---

A float's move handle is no longer a keyboard stop that does nothing: Tab passes it by. The new `TabViewer::on_window_handle` gives the application the handle's response, as `on_tab_button` does for tabs, so it can make it focusable and wire keys itself. Setting a shown float's `Place` in the layout from code now moves its window there, over the main window and natively, with no `FloatEvent::Moved`; it was only read when the window opened.
