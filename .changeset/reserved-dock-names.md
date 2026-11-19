---
egui_stack: patch
---

Documented: dock names `float-<digits>` and `window-<digits>` are reserved for the docks a `DockSpace` makes itself for floats (`float-N` for tabs moved out to a new float, `window-N` from `DockSpace::windows_to_floats`). The Move group menu recognises such a dock by its name and shows it by its float's window title where `TabViewer::dock_title` gives none (#150), so a dock the application names so is shown the same way. See `DockSpace::add_dock`.
