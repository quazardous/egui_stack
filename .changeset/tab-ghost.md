---
egui_stack: minor
---

A dragged tab leaves its bar and shows the same ghost as a dragged group, with just that tab: shadow, translucency, tab bar, body hint and size limits, painted by the new `DockPainter::dragged_tab`, whose default paints it as `dragged_group` paints a one-tab group (`DraggedGroupPaint` has a new field, `body_painted`). In a space, the window under the pointer paints it, and the window the drag began in paints nothing of it meanwhile. A tab dragged by the body of a leaf whose tab bar is hidden gets the ghost too.
