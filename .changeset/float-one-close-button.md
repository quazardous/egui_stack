---
egui_stack: patch
---

A float over the main window holding one tab group shows one close button on its top tab bar, its "Close window" beside the move handle: the group's close all button, which drew a second cross left of the handle since 0.28.9, is left out there, as closing all the group's tabs would close the window too. A float of several groups keeps each group's close all button.
