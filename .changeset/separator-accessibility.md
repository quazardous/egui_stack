---
egui_stack: minor
---

A `DockArea` separator is named after what it resizes, "Resize between Left and Right" (the titles of the active tabs on each side, of the first leaf for a side holding a split), from the new label `Labels::separator_between`, so a screen reader tells two separators apart; `Labels::separator` ("Resize") still names a grid's lines and a separator next to an empty leaf. A `LayoutArea` sash now exposes its step, `LayoutStyle::key_step`, next to its value, minimum and maximum in points, as a `DockArea` separator does. Both stay Tab stops moved by the arrow keys, and ranges set or stepped through accesskit (SetValue, Increment, Decrement). An app or test that looked a dock separator up by the name "Resize" finds it by its new name.
