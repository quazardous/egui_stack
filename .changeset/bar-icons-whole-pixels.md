---
egui_stack: patch
---

The icons of a tab bar's buttons (add tab, close all tabs, close window, move window, the float's close button) are drawn one point wide rounded to whole physical pixels of their window, so they stay sharp at a fractional scale such as 1.25 or 1.5 (a native float on a screen of its own included) instead of blurring over two pixel rows.
