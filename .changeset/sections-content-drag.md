---
egui_stack: patch
---

In a stack sized by its content (`Sections::fill(false)`), a drag moves a band again when the open sections above are at their content's height: down, the section above grows past its content, into the room under the last band; up, it shrinks to its least height. The last open section keeps the height a drag gives it, and the new `plan::fit_content` and `plan::pin_plan_content` lay such a stack out.
