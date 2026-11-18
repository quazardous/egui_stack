---
egui_stack: minor
---

The application shapes the docks of its floats: `FloatsArea::painter` paints them with its own `DockPainter` (tab bars and window outlines), `FloatsArea::configure` sets each float's `DockArea` options (no splits, no close or add buttons), and `FloatsArea::decorate` draws over each float once its dock is drawn, knowing where its tab bars are (`FloatShown`, also `DockArea::tab_bars`). `DockSpace::carry_floats_from` keeps the floats' windows and the drag in progress across a space rebuilt from the application's own layout.
