---
egui_stack: minor
---

`Docked<Tab>` keeps a `Layout` and its `DockSpace` as one value (saved together with the `serde` feature), and `Docked::repair` makes them agree: zones showing a dock the space lost go, docks no zone shows get a hidden zone. `Layout::from_space` builds a layout for a `DockSpace` placed by egui panels, from a tree shape the application gives.
