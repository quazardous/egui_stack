---
egui_stack: minor
---

A new `sections` module: `Sections`, a stack of collapsible sections filling a panel, each a title band over its own scrolling body (anything, windowed rows or the application's own view), folded by a click or the keyboard, resized by dragging the line above a band (sections give way in turn, the nearest first), with an optional pull that moves the sections below as a block, titles that drag or stay in place, accessible names and roles, and a `Layout` remembered in egui's memory by the stack's name (persisted with the `serde` feature); `sections::plan` holds its egui-free arithmetic. `DockPainter` gains `section_band` and `section_boundary`, with defaults painted from egui's `Style`, and `Labels` gains `resize_section`.
