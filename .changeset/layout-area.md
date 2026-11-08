---
egui_stack: minor
---

`layout::LayoutArea` draws a `Layout` in a `Ui`: each leaf zone's content through a `ZoneViewer`, as a pane named after the zone, and a sash between neighbouring zones that resizes the zone away from the one that grows, by drag, by the arrow keys (Shift for a larger step) or through assistive technology (a splitter whose value is the zone's size), within the zone's least and largest sizes and the growing zone's least size; Enter or a double click gives the zone back its default size. Zones with `fit_content` measure their content for the next frame. A `LayoutPainter` trait (with `DefaultLayoutPainter`) and a `LayoutStyle` let the host style the sashes; `Laid` lists the user's changes as `LayoutEvent`s, and each `LaidZone` its bounds, content and drop rule.
