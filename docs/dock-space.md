# Dock spaces: several docks, one drag

Status: design of the first step, implemented with it.

## The problem

A `DockArea` shows one `DockState`: one main tree and its floating windows. An application with
side areas (left, right, bottom around a centre) draws one `DockArea` per area, each inside an
egui panel. A tab dragged from one area to another then leaves egui_stack's drag handling: the
application has to follow the drag itself, guess which tab is dragged, and can only append the
tab to the target's tab bar, with no split, no insertion point and its own preview.

## The shape

A `DockSpace<Tab>` holds several **named docks**, each with its own tree, and the floating
windows, in one `DockState`. Every dock is a surface of that state (`Surface::Main`, the kind the
main surface already has), so a `TabPath`, a `NodePath` and a `TabDestination` already name a
place in any dock, and `DockState::move_tab` already moves a tab between any two of them.

```rust,ignore
let mut space = DockSpace::new();
space.add_dock("left", DockRules::default().splits(AllowedSplits::TopBottomOnly), vec![files]);
space.add_dock("centre", DockRules::default(), vec![editor, preview]);
space.add_dock("right", DockRules::default().keep_when_empty(true), vec![]);
```

### Drawing: one call per dock, one drag

egui is immediate: an application draws its side areas inside panels, at different points of the
frame, and decides their rects (width kept, folded, a flyout over the centre). So each dock is
drawn by its own call, inside the `Ui` the application gives it, and the docks share the drag
through the space:

```rust,ignore
egui::Panel::left("left").show(ui, |ui| {
    DockArea::dock(&mut space, "left").show_inside(ui, &mut viewer);
});
egui::CentralPanel::default().show(ui, |ui| {
    DockArea::dock(&mut space, "centre").show_inside(ui, &mut viewer);
});
```

All the calls of a space use the space's id (`DockSpace::id`), so they share one drag state in
egui's memory. egui_stack already resolves a drag one frame late: leaves record what is dragged
and what is hovered while they are drawn, and the next frame reads it before drawing. In a space,
the **first** call of a frame does that reading for every dock: it resolves the destination, paints
the preview, applies the drop on release, and the later calls of the frame only draw. Each call
records its dock's rect, which is what "outside every dock" is tested against.

One call, the space's windows dock (the first dock added, or the one named with
`DockSpace::set_windows_dock`), also draws the floating windows.

### Drop zones across docks

Nothing is specific to a pair of docks: the hovered leaf, in whichever dock, gives the same zones
as inside one dock (centre, the four edge bands, a slot in its tab bar), the same preview through
the `DockPainter`, the same overlay types. Dropping applies `DockState::move_tab` to the target
dock's tree: append, insert at an index, or split. An empty dock that is kept is a target as a
whole: dropping on it puts the tab in it.

### Rules per dock

`DockRules` (serialised with the space):

| Rule | Default | Effect |
|---|---|---|
| `splits` | `AllowedSplits::All` | The splits a drop may make in this dock, on top of `DockArea::allowed_splits`. |
| `windows` | `true` | Whether a tab dragged out of this dock may become a floating window. |
| `keep_when_empty` | `true` | Kept as an empty drop target once its last tab leaves; otherwise removed from the space. |

Where a dock is and how big it is stays the application's: egui_stack only needs each dock's rect
each frame, which is the `Ui` the dock is drawn in.

### Vetoes

`TabViewer::drop_allowed(tab, destination)` stays the veto. A space adds
`TabViewer::dock_drop_allowed(tab, &DockDrop)`, which names the docks (`from`, `to`; `None` for a
floating window) beside the destination; by default it asks `drop_allowed`, so a viewer overrides
either one. It is asked inside one dock and between docks alike, and a refusal shows on the
preview with its reason, as today.

### Floating windows

They are the space's, not a dock's: a tab may go from any dock to a window and back to any dock,
unless the dock it leaves has `windows: false` or the viewer's `allowed_in_windows` says no.

### Serialisation

With the `serde` feature, a `DockSpace` serialises as one value: its `DockState` (every dock's tree
and the windows), its docks (name, surface, rules) and its windows dock. Surface indices are
stable: emptied windows leave a hole, as `DockState::remove_surface` already did, rather than
shifting the docks after them.

Migration from several `DockState`s, one per area: `DockSpace::from_states` takes them with their
names and rules; each main tree becomes a dock, and their windows become the space's.

## What stays as is

- `DockArea::new(&mut DockState)` and every existing call: a single dock, unchanged.
- `DockState`, `Tree`, `TabViewer::drop_allowed`, `DockPainter`, the overlay types.
- `TabPath`, `NodePath`, `SurfaceIndex`: a dock is a surface, named by the space.

## Later

- Detached OS windows as surfaces of the space (a host hook opening the viewport).
- A hook letting the application decide a move between docks (a step model's veto or redirect)
  after the drop, beyond the verdict shown during the drag.
