---
egui_stack: minor
---

`DockSpace::no_drop_zone`, `DockSpace::no_drop_zones` and `DockSpace::no_drop_widget` declare, every frame, where no dragged tab may be dropped (an application's toolbar or status bar): a tab released there goes back where it was instead of becoming a floating window, and no drop preview shows there. `DockRules::accept_drops(false)` makes a dock of a space no drop target, its own tabs still free to leave it.
