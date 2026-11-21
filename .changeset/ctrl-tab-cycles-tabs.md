---
egui_stack: minor
---

Ctrl+Tab and Ctrl+Shift+Tab cycle the active tab of the tab group under the pointer, next and previous, wrapping around, as a click selects a tab: the innermost group cycles, in the docks and in the floats (over the main window or native), and a focused text edit keeps the keys. On by default; `DockArea::cycle_tabs_with_keyboard(false)` and `FloatsArea::cycle_tabs_with_keyboard(false)` turn it off. Taken from egui_tiles' `Behavior::cycle_tabs_with_keyboard` (https://github.com/rerun-io/egui_tiles).
