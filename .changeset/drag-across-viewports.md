---
egui_stack: minor
---

A tab or a whole group is dragged across native windows in one drag (`DockSpace::dragging`, `SpaceDrag`, `DragPayload`): from the main window to a native float, back, or from float to float, with the same drop zones, veto (asked when the destination changes and at release) and Escape, across screens of different scales; released over nothing it opens a new float, a native float's tab torn off included; closing the window under the pointer cancels the drag; where window positions are hidden (Wayland), the windows that could take it outline their docks (`DockPainter::window_outline`, `WindowOutlinePaint`) and the first to see the pointer after the release takes it. A tab that moves into a float's dock records its origin, and forgets it back in a dock. `TabDestination` and `TabInsert` are `Clone`, `Copy`, `Debug` and `PartialEq`.
