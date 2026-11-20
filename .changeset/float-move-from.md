---
egui_stack: minor
---

`FloatsArea::move_from` sets where else than its move handle and title bar a drag moves a float: `MoveFrom::EMPTY_SPACE`, a drag on the part of the float no widget takes a drag from (the empty part of a tab bar, which then no longer drags its group; the body's background), and `MoveFrom::modifier(Modifiers::ALT)`, a drag begun with the modifiers held anywhere in the float, over its widgets too; combine them with `|`. By default `MoveFrom::BAR`, as before. The move goes as one by the handle: `FloatsArea::float_move` says whether it can dock the float.
