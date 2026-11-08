# Examples

This directory contains examples of how to use `egui_stack`.

To run an example, use the following command from the project root:

```bash
cargo run --example <example_name>
```

Replace `<example_name>` with the name of the example file without the `.rs` extension.

## List of Examples

- [`hello`](hello.rs): A comprehensive demo of `egui_stack` features, including a style editor and various dock configurations.
- [`simple`](simple.rs): A minimal example showing how to set up a basic dock with a few tabs.
- [`dock_space`](dock_space.rs): Three docks, left, centre and right, each in its own panel, sharing one drag; a veto keeps the editor in the centre.
- [`layout`](layout.rs): A whole window laid out in zones (a toolbar, two sides around a centre that keeps its least size, a status bar), resized by sashes.
- [`custom_painter`](custom_painter.rs): Tabs and separators painted by a `DockPainter` instead of the `Style`.
- [`tab_add`](tab_add.rs): Shows how to implement a custom "add tab" button and handle it in the `update` loop.
- [`tab_add_popup`](tab_add_popup.rs): Demonstrates how to use a popup menu for adding different types of tabs.
- [`save_load_dock_state`](save_load_dock_state.rs): Shows how to persist the dock layout to a JSON file and load it back.
- [`text_editor`](text_editor.rs): A practical example of a text editor with multiple open buffers as tabs.
- [`reject_windows`](reject_windows.rs): Demonstrates the ability of tabs to either allow or reject being moved into separate windows.
- [`sections`](sections.rs): A stack of collapsible, resizable sections in a side panel.
- [`tool_row`](tool_row.rs): Toolbar rows: a formatting row whose tools overflow into a More button, and a vertical tool palette.
