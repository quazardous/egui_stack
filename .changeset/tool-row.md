---
egui_stack: minor
---

A toolbar row, `toolbar::ToolRow`: a row or a column of tools the caller gives (`Tool::button`, `toggle`, `radio`, `menu`, `separator`, `spacer` and `custom`), showing icons, labels or both (`ToolDisplay`), with tooltips and key bindings (`Tool::shortcut`), the tools that do not fit going in their order into a More button that opens them as a menu. It says which tool was clicked and holds no state of its own. It is a named `Toolbar` for screen readers, with toggles announced as toggled and busy tools as busy, and one Tab stop with the arrows, Home and End along it (`TabFocus::Roving`, its default). `toolbar::laid_out`, `focused` and `focus` read the row as drawn and move the focus in it. It is painted by the `DockPainter`, through three new methods with defaults, `tool_item`, `tool_more` and `tool_separator`; `PartState::DISABLED` is new, and `Labels` has two new names, `more_tools` and `more_tools_in`.
