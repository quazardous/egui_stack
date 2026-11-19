---
egui_stack: patch
---

The Move group menu no longer shows a float's dock the space named itself (`float-N`, made when tabs are moved out to a new float) by that name: where `TabViewer::dock_title` gives no title, the item reads the float's window title, the active tab's title of its focused (else first) group, and follows it when the active tab changes. A dock the application named keeps its name, and `dock_title` still takes precedence (#150).
