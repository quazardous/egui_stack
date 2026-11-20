---
egui_stack: patch
---

The scroll handle of a tab bar whose tabs overflow is keyed by its surface too: two docks of a space, or a dock and a float, each with an overflowing bar on a leaf of the same index, no longer share an id (egui's debug assertion on a widget changing layers fired when a group was moved into a float).
