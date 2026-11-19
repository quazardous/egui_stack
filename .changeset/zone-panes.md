---
egui_stack: patch
---

A `LayoutArea` zone's content now lies inside its pane in the accessibility tree: the content's `Ui` is the `Pane` named after the zone's title, where it was an empty `Pane` beside its controls. A zone lying over its neighbour does the same, and a flyout's content is a `Pane` named by `ZoneViewer::flyout_title`.
