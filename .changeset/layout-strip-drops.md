---
egui_stack: minor
---

`Constraints::strip_drops`: a folded zone's strip takes a dragged tab for the dock it holds, which joins that dock's focused leaf (else its first), through `DockSpace::bind`. Off by default: strips still refuse drops.
