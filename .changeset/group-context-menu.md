---
egui_stack: major
---

A right click on the empty part of a tab bar, in a dock or a float, opens the group's menu: what `TabViewer::group_context_menu` adds, then "Move group", "Eject group" (where the group can float out), "Add tab" (with add buttons) and "Close all tabs" (with the close all buttons). A right click on a tab still opens the tab's menu. `DockArea::group_context_menus(false)` turns it off. `TabContextMenuTranslations` has three new fields for its items, `eject_group_button`, `add_tab_button` and `close_all_tabs_button`. Migration: a `TabContextMenuTranslations` built field by field sets the three new fields, or ends with `..TabContextMenuTranslations::english()`.
