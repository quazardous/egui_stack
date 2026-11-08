---
egui_stack: minor
---

`DockSpace::bind` puts a dock space on a layout: called each frame with the `Laid` a `LayoutArea` returned, it makes no-drop zones of the zones with `Drops::Refuse`, the zones holding no dock (`Content::App`, or a dock the space has not) and the folded zones' strips, so a tab released there goes back where it was; a `Content::Empty` zone that accepts drops stays outside every dock, where a tab becomes a floating window. Each dock is drawn in its zone by the `ZoneViewer`, with `DockArea::dock`.
