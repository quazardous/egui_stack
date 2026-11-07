---
egui_stack: minor
---

`Section::max_height` gives a section a greatest height, in points, and `Section::max_share` one as a share of the stack's height: sized by the list or set by hand, the section never grows past it, the room it leaves going to the other sections, or staying empty under the last band when none can take it, and a drag on its boundary stops there. A greatest height under the section's least height is its least height.
