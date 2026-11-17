---
egui_stack: minor
---

An editor whose tab bar is hidden can be moved again by a handle of the app's choosing: `DockArea::drag_group_by(ui, &response)`, called from `TabViewer::ui` with the response of a widget that senses drags (an editor's own header, a grip), makes that widget drag the whole tab group, as the empty part of a tab bar does, through the same drop zones and past `OverlayFeel::drag_activation_distance`. `DockArea::hidden_tab_bar_drags_group(true)` makes the strip at the top of a hidden tab bar's leaf (`TabBodyStyle::hidden_tab_bar_drag_height` high) drag the group too, instead of its active tab alone. Both are opt-in: nothing changes for an app that uses neither.
