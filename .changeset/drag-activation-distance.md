---
egui_stack: minor
---

`OverlayFeel` has a new field, `drag_activation_distance`: how far, along `x` and `y`, the pointer moves from where it pressed a tab (or the empty part of a tab bar) before the tab (or the tab group) starts being dragged, measured from the press. It is `6.0` along both axes by default, egui's click distance, so a tab drags as soon as the press is no longer a click, in every direction; it used to start only 30 points to the side or 6 points up or down past that point, about 38 and 14 points from the press, with a range in between where the tab neither clicked nor dragged. `vec2(37.0, 13.0)` gives the former distances back.
