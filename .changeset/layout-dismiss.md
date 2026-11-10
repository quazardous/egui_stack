---
egui_stack: minor
---

`LayoutArea::flyout_dismiss` and `LayoutArea::over_dismiss` take a `Dismiss` saying what closes the flyout and what folds back a zone lying over its neighbour: a press outside, Escape (`Dismiss::ANY`, the default as before), one of them, or neither (`Dismiss::NEVER`, lying over then lasts until its grip folds it). `LayoutArea::inside` lets the host count a press outside as inside, from its point and the egui layer on top there, so that a combo box's list, a context menu or the host's own overlays keep the flyout open. A host that closes its own overlays first on Escape turns `escape` off or consumes the key before the area is drawn.
