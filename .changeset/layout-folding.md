---
egui_stack: minor
---

A `LayoutArea` folds zones: a folded zone keeps a strip in the flow, painted by `LayoutPainter::strip` with a grip that unfolds it, and the application's icons from `ZoneViewer::strip`; a sash dragged under half a `Fold::ToStrip` zone's least size folds it on release. A zone may lie over its neighbour (`Layout::set_over`), next to its strip and without taking room, until Escape or a click outside folds it back. `LayoutArea::flyout` shows one `Flyout` of a folded zone over its neighbour (`ZoneViewer::flyout`), closing on Escape, a click outside or the grip (`LayoutEvent::FlyoutClosed`), with a free edge that resizes it; the breadths are kept per key in `Layout::flyouts`, saved with the layout as the zones' states are. `Layout::set_folded` and `set_over` change a zone's state from code.
