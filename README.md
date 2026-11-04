# `egui_stack`: docking for [egui](https://github.com/emilk/egui)

[![github](https://img.shields.io/badge/github-quazardous/egui__stack-8da0cb?logo=github)](https://github.com/quazardous/egui_stack)
[![egui_version](https://img.shields.io/badge/egui-0.37-blue)](https://github.com/emilk/egui)

`egui_stack` continues [`egui_dock`](https://github.com/anhosh/egui_dock): a docking system for `egui`, originally
created by [@lain-dono](https://github.com/lain-dono) and developed for years by
[Adam Gąsior (@Adanos020)](https://github.com/Adanos020), later maintained by [@anhosh](https://github.com/anhosh), with
many contributors. Its full history is kept here, so every commit keeps its author. Thanks to all of them.

It is a divergent fork: the API starts as `egui_dock` 0.21.1's, moved to egui 0.37, and will grow to hold more of an
application's panel management (stackable sections, toolbars). See [NOTICE](NOTICE) and the
[changelog](CHANGELOG.md).

What differs from `egui_dock`:

- The crate is `egui_stack`; `use egui_dock::` becomes `use egui_stack::`.
- egui 0.37.
- No particular policy on AI-assisted contributions: they are reviewed like any other, under the same rules (see
  [CONTRIBUTING.md](CONTRIBUTING.md)).

## Features

- Opening and closing tabs.
- Moving tabs between nodes and resizing.
- Dragging tabs out into new `egui` windows.
- Highly customizable look and feel: a `Style`, or a `DockPainter` that paints the tabs, tab bars, separators and drop
  overlay itself.
- High degree of control over behaviour of the whole dock area and of individual tabs.
- Manipulating tabs and dock layout from code.
- Collapsible sections stacked in a panel (`sections::Sections`): fold, resize, pull as a block, remembered by name,
  painted by the same `DockPainter`.

## Quick start

Add `egui` and `egui_stack` to your project's dependencies. It is not on crates.io yet; until then, use Git:

```toml
[dependencies]
egui = "0.37"
egui_stack = { git = "https://github.com/quazardous/egui_stack" }
```

Then proceed by setting up `egui`, following its [quick start guide](https://github.com/emilk/egui#quick-start). Once
that's done, you can start using `egui_stack`; its documentation builds with `cargo doc --open`.

## Examples

The Git repository of this crate contains some example applications demonstrating how to achieve certain effects. You
can find all of them in the [`examples`](examples) folder.

You can run them with Cargo from the crate's root directory, for example: `cargo run --example hello`.

## Demo

![demo](images/demo.gif "Demo")

## Alternatives

### [egui_tiles](https://github.com/rerun-io/egui_tiles)

It's a library aiming to achieve similar goals in addition to being more flexible and customizable.

One feature it supports that `egui_stack` does not at the moment is the ability to divide nodes into more than two
children, enabling horizontal, vertical, and grid layouts.

