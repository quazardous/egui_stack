---
egui_stack: minor
---

`GridLayout::Auto` picks the number of columns whose cells come closest to 4:3 in the grid's rectangle, so three views in a wide strip make one row instead of two rows with a hole, and the count follows the grid as it is resized; before its first layout a grid keeps the square root rule. The heuristic is egui_tiles' `grid_auto_column_count` (https://github.com/rerun-io/egui_tiles).
