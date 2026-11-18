---
egui_stack: patch
---

The buttons at the right end of a tab bar sit side by side again: with the add button after the last tab (the default since 0.28.2), its empty slot no longer leaves a gap between the group grip and the close-all button. The slot is now left of the grip, where the add button goes once the tabs fill the bar or with `TabAddAlign::Right`.
