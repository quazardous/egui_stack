---
egui_stack: minor
---

`LayoutPainter::over_edge` paints the free edge of a zone lying over its neighbour, the edge that resizes it, with its state (hovered, pressed, dragged) in a `SashPaint`, above the zone's content. By default it paints nothing, as before; a host that styles its sashes' hover and drag can paint the same line there, so a docked zone's sash and a zone lying over look alike.
