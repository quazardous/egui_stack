---
egui_stack: patch
---

A click anywhere on a folded zone's strip now unfolds it (or folds back a zone lying over), as its grip does, wherever the strip's own widgets do not take the click: an empty strip is a button as a whole. The band at the strip's edge that drags the zone out takes a press without motion as a click too, and on a strip narrower than twice `LayoutStyle::sash_grab` it lies just outside the strip instead of covering it, so the resize cursor shows only there.
