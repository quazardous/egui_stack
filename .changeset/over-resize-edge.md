---
egui_stack: minor
---

A zone lying over its neighbour resizes from its free edge, as a flyout does: a splitter named as its sash ("Resize left"), within its least and largest sizes, written to its `ZoneState::size` and reported as `LayoutEvent::Resized`.
