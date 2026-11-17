---
egui_stack: patch
---

A dragged tab lands where the pointer is released, not where it was a frame earlier: released on a leaf's tab bar it joins the leaf's tabs, released in an edge band it splits the leaf, also when the pointer moved there in the release frame or while the hovered leaf is still preferred (`OverlayFeel::max_preference_time`), which now keeps the leaf only, not whether its tab bar or its body was hovered.
