---
egui_stack: patch
---

A folded split zone's strip that takes no drops (without `Constraints::strip_drops`, or when none of its docks accepts them) is a no-drop zone for `DockSpace::bind`, as a folded leaf's strip is: a tab released there stays where it was, instead of opening in a floating window.
