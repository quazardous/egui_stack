---
egui_stack: patch
---

A tab or group drag whose release the window never sees ends as a cancel, as on Escape: the layout stays as it was and the ghost goes, in one window or across the viewports of a `DockSpace`. On Wayland, a screenshot taken during a drag grabs the pointer and the button is let go while the compositor holds it; the drag stayed live, its ghost following the pointer. It is now cancelled when the window loses the focus, or when the button is pressed again while egui still believes it held; the drag stays stopped until the button is let go.
