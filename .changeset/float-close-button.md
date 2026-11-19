---
egui_stack: patch
---

A float over the main window without a title bar of its own (the default since 0.28.7) has a
"Close window" button beside its move handle: it closes the float as the title bar's close
button does, asking `TabViewer::close_policy` what becomes of its tabs, even when they cannot be
closed one by one, which left such a float with no way to close it (#143).
