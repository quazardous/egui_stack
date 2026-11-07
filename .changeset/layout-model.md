---
egui_stack: minor
---

A new `layout` module models a window's whole space as a tree of zones (`Layout`, `Zone`, `Content`), with constraints set from code (least, largest and default sizes in points or shares, a zone that grows, priorities, folding) and the user's state, and solves it into rects (`Layout::solve`, `Laid`): the sides give way to the centre's least size, lowest priority and nearest first, never in proportion, and zones that opt in fold to a strip when even the least sizes do not fit. With `serde`, a layout saves its tree and state, versioned; `Layout::reconcile` re-asserts the code's constraints after loading and adds or drops the zones the code added or removed. Nothing is drawn yet.
