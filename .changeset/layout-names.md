---
egui_stack: minor
---

A `LayoutArea`'s zones are named after their title, `ZoneViewer::title` (by default the zone's name), and its sashes, grips and flyout edge after `Labels::resize_zone`, `unfold_zone`, `fold_zone` and `resize_flyout`, set with `LayoutArea::labels`, so an application translates them and its zones' names never clash with its own labels. `ZoneViewer::flyout_title` names a flyout.
