---
egui_stack: major
---

The layout holds floating roots, ahead of native floating docks (#105): `layout::Float` (a key, a dock, a host `FloatHost::InWindow` or `Native`, a `Place`, a z, minimized), `Layout::floats` and the places of closed floats (`Layout::closed`, `open_float` reopens where it was), `Screen` and the pure `Place::repair` (re-projected to the screen's scale, shrunk to fit, title bar kept on a screen, `ScreenGone::MoveOnto` or `DockBack` when its screen is gone), and `DockSpace::record_origin` keeping a tab's dock and neighbours (`TabOrigin`). Nothing draws floats yet. The saved layout is version 2: a `Docked` saved by 0.27 or earlier turns its floating windows into docks `window-N` shown by native floats, and a version 2 layout does not load in 0.27. Migration: a `Layout` built field by field adds `floats: Vec::new()` and `closed: BTreeMap::new()` (or use `Layout::new`).
