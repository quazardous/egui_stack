---
egui_stack: patch
---

A whole tab group being dragged (by its grip, the empty part of its tab bar, Shift and a tab, or a `DockArea::drag_group_by` handle) shows its ghost under the pointer, as a dragged tab does: its tab bar with its tabs, the active one marked, and a hint of its body, kept to a sensible size with a "+N" for the tabs that do not fit, in whichever window of a `DockSpace` the pointer is over. It is painted by a new provided method, `DockPainter::dragged_group`, from a `DraggedGroupPaint` (with `DraggedGroupTab` and `DraggedGroupMore`), so an application can restyle it.
