---
egui_stack: minor
---

A folded split zone's strip can take dragged tabs: with `Constraints::strip_drops` on a split zone, a tab released on its strip joins one of the docks the split holds that accept drops, the one holding the focused leaf (the last active), else the first, depth first, and `DockSpace::take_unfolds`, called after the layout is drawn, names the zone for the application to unfold with `Layout::set_folded`. `LaidZone` has a new field, `docks`: the docks a split's leaves hold. Off by default: a split zone without `strip_drops` behaves as before.
