---
egui_stack: minor
---

The floats' work area can tear out to native windows: with `WorkArea::outside(Outside::Native)`, a tab, a group or a float over the main window released past the work area's edge becomes a native float, opened where it was released, while the floats inside the work area stay bounded as with `Outside::Cancel`. It happens on release, with no window following the pointer during the drag. Where native windows are not available (`ViewportHost::native_windows`, false by default when egui embeds its viewports), it is `Outside::Cancel`. A native float's docks still take drops (#148).
