---
egui_stack: minor
---

A whole tab group can be dragged by the empty part of its tab bar, as an editor group is dragged by its tab bar: all its tabs move together, in order, its active tab kept, with the same drop zones as a tab (the middle of a leaf joins its tabs, an edge band splits it, outside the dock a new window), and the viewer's `dock_drop_allowed` asked for each of its tabs. `DockArea::dragged_node` reports the drag (`DraggedNode`), and `DockState::move_node` moves a leaf whole as `move_tab` moves a tab.
