---
egui_stack: patch
---

A float over the main window whose tab groups are all collapsed shrinks to its tab bars, and hides its resize grip; expanding a group gives it back the height it had, which its place keeps meanwhile, so a layout saved collapsed expands to it too. It kept its full height before, its body empty. A native float keeps its window's size, the system's to set.
