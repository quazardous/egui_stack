---
egui_stack: patch
---

A docked zone's content follows its sash in the same frame while the sash
is dragged: `LayoutArea` applies the sashes' moves before it draws the
zones, and paints the sashes from the same layout, so the content's edge no
longer trails the sash line by one frame.
