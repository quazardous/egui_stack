---
egui_stack: minor
---

Tab bars can show an Eject group button, `DockArea::show_leaf_eject_buttons` (and `FloatsArea::show_leaf_eject_buttons` for the floats' docks), off by default: clicked, it moves the whole group, its tabs in order, into a new floating window at the group's place, as "Move group" then "New window" does. A group already alone in its window or float has none, nor a group with a tab kept out of windows. Its accessible name is `Labels::eject_group`, "Eject group". An application adds buttons of its own to a group's bar with `TabViewer::group_bar_buttons` (their accessible names) and `TabViewer::on_group_bar_button` (the group's path and each button's `Response`).
