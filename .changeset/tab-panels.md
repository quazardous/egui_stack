---
egui_stack: minor
---

Dock tabs follow the accessible tab pattern: the active tab's body is a `TabPanel` under its leaf, named by its tab, which `controls` it, and each tab list has a name, the dock's title (`TabViewer::dock_title`) or the new `Labels::tab_list` ("Tabs" by default) to translate. Without its tab bar, the panel takes the tab's title as its name.
