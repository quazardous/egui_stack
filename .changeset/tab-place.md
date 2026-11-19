---
egui_stack: minor
---

A tab can leave a `DockSpace` and come back where it was, to hide and show it or to close and reopen it in place: `DockSpace::take_with_place` removes a tab and returns it with its `TabPlace` (its dock, the tabs next to it and its index in its group; opaque, and serialisable with the `serde` feature), and `DockSpace::restore_at` puts it back, beside its neighbours, else in its dock's focused group at its old index, else where a float's tab goes when the float closes (`ClosePolicy::ReturnToOrigin`). No hidden tab stays in the tree (#141).
