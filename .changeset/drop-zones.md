---
egui_stack: major
---

A new drop overlay, `OverlayType::Zones`, is the default: while a tab is dragged over a leaf, the region it would take (the half along an edge when the pointer is in that edge's band, the whole leaf elsewhere) is drawn on the leaf itself with a border and a light tint, with no buttons; dropped outside the dock, a tab allowed in windows opens in a new window. Its look is `OverlayStyle::zone_fill`, `zone_stroke` and `zone_corner_radius`, the width of the edge bands `OverlayFeel::zone_edge_coverage`, and `DropZone::at` is its hit-test, public for applications drawing their own drop targets. `OverlayType` is now `#[non_exhaustive]`. Migration: a `match` on `OverlayType` needs a `_` arm; a `Style`, `OverlayStyle` or `OverlayFeel` built field by field takes the new fields (or `..Default::default()`); to keep the former overlay, set `style.overlay.overlay_type = OverlayType::Widgets`.
