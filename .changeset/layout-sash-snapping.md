---
egui_stack: minor
---

A dragged layout sash snaps to its zone's default size and to the line of another sash running the same way, within `LayoutStyle::snap` (8 points; 0 turns it off), with a guide line painted by `LayoutPainter::snap_guide` (`SnapGuidePaint`, `SnapKind`); Alt suspends it. A guide also shows where a zone will fold on release.
