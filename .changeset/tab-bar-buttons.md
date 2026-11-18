---
egui_stack: patch
---

Every button of a tab bar paints the same way, through one helper: the line under the bar now runs under the collapse button and a float's move handle too (it stopped short of them), no button's highlight covers it any more (the group grip's, the collapse button's and the move handle's did), and the move handle has the left border the add and close all buttons have. The group grip and the collapse button's symbol sit a point higher, centred above that line like the others'; `DockPainter::tab_bar_tail` is also called for the collapse button's slot, and `TabBarGripPaint::rect` leaves out the line.
