---
egui_stack: minor
---

`Sections::fill(false)` sizes a stack by its content: an open section sized by the list is never taller than what its body laid out, the long ones share what is left, the folded bands follow right under the last open section and the room left stays empty under the last band; sections set by hand, resizing, least heights and scrolling are as before. `sections::plan::fit_within` and `pin_plan_within` are its arithmetic.
