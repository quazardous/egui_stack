---
egui_stack: minor
---

The corners of a tab bar follow what is below it: a folded leaf, which shows only its tab bar, has its bar and tabs rounded at the bottom, with no line under them, and so does a dragged tab, while a bar over a body stays square at the bottom whatever the state of its tabs. The highlights of a tab's close button and of the bar's buttons (collapse, add, close all) are rounded where they meet a rounded corner of the tab or bar instead of sticking out square, and the close button's highlight no longer covers the tab's outline. `TabBarPaint`, `TabBarTailPaint` and `TabPaint` have a new field, `folded`, which tells a painter that the leaf is folded.
