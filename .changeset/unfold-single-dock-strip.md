---
egui_stack: major
---

Behaviour change: `DockSpace::take_unfolds` now also reports a folded zone holding a single dock
when a tab is dropped on its strip (`Constraints::strip_drops`), as it already did for a folded
split zone; before, such a drop was not reported. egui_stack still unfolds nothing itself: an
application that unfolds every zone `take_unfolds` returns now unfolds single-dock zones on a
strip drop too, and one that wants them to stay folded leaves them out. With the `bus` feature,
the drop names the zone as well as the dock: `DropAsk` has a new field, `zone` (the folded or
collapsed zone dropped on, else the zone holding the destination dock), and the `TabMoved`
`LayoutChange` sets `zone` the same way (#147). Migration: apps that unfold everything
`take_unfolds()` reports now also unfold single-dock zones; leave those zones out to keep them
folded.
