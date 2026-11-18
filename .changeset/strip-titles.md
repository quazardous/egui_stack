---
egui_stack: minor
---

Folded strips can show their zone's title: set `LayoutStyle::strip_title` to a `StripTitle` (off by default). Left and right strips read the same way, from the bottom up unless `StripTitle::vertical` says `VerticalReading::TopToBottom`; a long title is cut with an ellipsis and shown whole in a tooltip; the strip grows to fit the font, an optional icon (`ZoneViewer::strip_icon`) and the padding. `ZoneViewer::strip_title(ui, zone, rect, orientation)` draws the title instead, in the room past the grip, while the layout keeps the strip's clicks, drag and drops.
