---
egui_stack: minor
---

`LayoutArea::sublayer_of(parent)` draws the zones lying over their neighbour and the flyout as sublayers of `parent`, a foreground layer the host holds: egui keeps them directly above it whatever was clicked last, so a host's own overlays shown above that layer stay above them. By default they are plain foreground areas, raised when clicked, as before.
