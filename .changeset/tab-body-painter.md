---
egui_stack: minor
---

A leaf's body border is painted by the `DockPainter`, in a new provided method, `tab_body`, which gets a `TabBodyPaint`: the body's rect, its corner radius, the styles, and its state (`PartState::HOVERED` under the pointer, `PartState::FOCUSED` and `PartState::LEAF_FOCUSED` when its leaf is the focused one). A painter can then draw borders no style field gives: on some sides only, glowing, or on the focused leaf alone. `StylePainter` paints `TabBodyStyle::stroke` as before.
