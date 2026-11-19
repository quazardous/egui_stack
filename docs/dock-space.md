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
| `accept_drops` | `true` | Whether a tab may be dropped in this dock. Off, the dock is never a drop target, its own tabs included; they can still be dragged out. |

Where a dock is and how big it is stays the application's: egui_stack only needs each dock's rect
each frame, which is the `Ui` the dock is drawn in.

### Vetoes

`TabViewer::drop_allowed(tab, destination)` stays the veto. A space adds
`TabViewer::dock_drop_allowed(tab, &DockDrop)`, which names the docks (`from`, `to`; `None` for a
floating window) beside the destination; by default it asks `drop_allowed`, so a viewer overrides
either one. It is asked inside one dock and between docks alike, and a refusal shows on the
preview with its reason, as today.

### No-drop zones

A tab released outside every dock becomes a floating window where it lands. An application keeps
parts of its window out of that, its toolbar or its status bar, by declaring them every frame:

```rust,ignore
space.no_drop_zone(toolbar_rect);
space.no_drop_zones([status_bar_rect, ruler_rect]);
space.no_drop_widget(menu_bar_id); // its rect as egui last saw it
```

A tab released in a zone goes back where it was: it does not float, and no drop preview shows
there. A zone wins over a dock under it; a floating window over it still takes the tab. The zones
last one frame: the space forgets them once its docks have read them, so an application declares
them each frame, before or after the docks. A dock with `accept_drops: false` refuses drops the
same way, through the same test.

### Floating windows

They are the space's, not a dock's: a tab may go from any dock to a window and back to any dock,
unless the dock it leaves has `windows: false` or the viewer's `allowed_in_windows` says no.

A float's dock the space makes itself is named `float-N` (tabs moved out to a new float) or
`window-N` (a former egui_dock window, `DockSpace::windows_to_floats`). These names, `float-` or
`window-` followed by digits only, are reserved: the space recognises such a dock by its name and,
where `TabViewer::dock_title` gives no title, shows it by its float's window title (the active
tab's title of its focused, else first, group) rather than by its name, in the Move group menu
for one. A dock the application adds under such a name is shown the same way, so an application
names its own docks otherwise (`float`, `float-tools`, `window-a` are fine).

### The floats' work area

By default the floats over the main window go anywhere in it, and a tab released outside every
dock floats where it lands. `FloatsArea::work_area` keeps both inside regions of the window,
resolved by egui_stack every frame, so the application computes no geometry:

```rust,ignore
docked
    .floats()
    .work_area(WorkArea::new().zone("centre").zone("right")) // Outside::Cancel by default
    .show(ctx, &mut viewer);
```

A region is the whole space as drawn (`.space()`: the root zone `DockSpace::bind` last read, else
the union of the docks drawn in the main window), a zone of the layout, leaf or split
(`.zone(name)`, as `bind` last read it: zones are the layout's only named regions), or a rect
(`.rect(r)`). A zone folded, hidden or collapsed is no region until it opens again; with no region
left the floats are free. `WorkArea::window()`, the default, is the whole window.

- A tab or a group released outside every dock and every region is refused through the no-drop
  path (`Outside::Cancel`): it stays where it was. Docks, strips and restore bands keep their rules.
- A float over the main window stays in one region, the one it overlaps most. Moved, it goes to
  the region under the pointer, if any, else stays in its own: over a gap it does not follow the
  pointer, so it never straddles two regions and does not flicker at a boundary. egui's
  `constrain_to` bounds its position; its size is capped by the region too (`max_size`; egui 0.37
  also caps a window to its constrain rect), so a float larger than its region shrinks.
- A region that shrinks or goes, or a layout restored with a float outside every region, brings
  the float into its region or the nearest one, reported once as `FloatEvent::Moved`.
- Native floats ignore the work area: they are free on the desktop.

The setting is an option of `FloatsArea`, given every frame like its other options, and kept by
the space (not saved) for its docks: they decide a drop while they are drawn, before the floats,
so they read the value the floats were given last frame. Only the very first frame has none yet,
which no drag notices, as a drag takes several frames; a `DockSpace::set_work_area` would avoid
that frame at the cost of a second place to configure the floats, so the option stays with the
floats. Zone rects come from `DockSpace::bind`, as strips and no-drop zones do.

Leaving the window for a native float past a region's edge (`Outside::Native`, as Visual Studio or
Blender tear tabs out) is a later step.

### Moving a whole tab group

A tab group (a leaf with all its tabs) moves as one, through the same zones, veto and Escape as a
single tab, its tabs kept in order and its active tab kept. `DockArea::group_handles` sets which
handles move it, a set of `GroupHandles` flags, all on by default:

| Flag | Handle |
|---|---|
| `EMPTY_SPACE` | The empty part of the tab bar, right of the last tab. A bar full of tabs has none. |
| `GRIP` | A grip at the end of each tab bar, always there however many tabs it holds. Named "Move group" for screen readers (`Labels::move_group`), painted by `DockPainter::tab_bar_grip`. |
| `MODIFIER_DRAG` | A tab dragged with `DockArea::group_drag_modifiers` held, Shift by default, carries its group. Shift has no other meaning on a tab drag: egui gives none, Shift on the tab bar's buttons is a click, and Ctrl (Cmd) is a `Sections` pull. |
| `MENU` | "Move group" in the tab's context menu, and the grip's menu (clicked, right-clicked or opened from the keyboard): a new window and each other dock of the space that takes drops. A place the viewer refuses is greyed, its reason in a tooltip. |

An application narrows the set to what it allows; a user setting narrows it further with
`GroupHandles::intersection`. A handle that is off is not drawn and does not react.
`DockArea::drag_group_by` and `DockArea::hidden_tab_bar_drags_group` are handles the application
asks for itself, outside these flags.

In a floating window, a drag does what the place it starts says: a tab drags the tab, the group
grip or the group modifier drags the group, and the window's move handle (or its title bar, when
it has one: a native float's, or one over the main window with `FloatsArea::title_bar`) moves
the window. The empty part of a tab bar never moves the window (it drags the group when
`EMPTY_SPACE` is on).
One more flag is a floats' handle:

| Flag | Handle |
|---|---|
| `MOVE_WINDOW` | A move handle at the right end of a float's top tab bar, apart from the grip. It moves the window, with the move cursor: through the system on a native float (`ViewportCommand::StartDrag`), as the `egui::Window` on a float over the main window. Named "Move window" (`Labels::move_window`). |

`FloatsArea::group_handles` and `FloatsArea::group_drag_modifiers` set them for the floats'
docks; `FloatsArea::float_move` sets what moving a window does (see
[native-floats.md](native-floats.md#float-gestures)).

The add tab button sits right after the last tab (`ButtonsStyle::add_tab_align`,
`TabAddAlign::Left` by default); `TabAddAlign::Right` puts it at the end of the bar. Its slot is
reserved left of the grip, so the buttons at the end of the bar (the add button once it is there,
the grip, the close-all button) always sit side by side.

### Taking a tab out and putting it back: `TabPlace`

A space has no hidden tab: a tab in a tree is drawn, counted, focused, dragged, listed in menus,
saved and read by screen readers. To hide a tab, or to close one and reopen it where it was, the
application takes it out of the space with its place, and keeps both:

- `DockSpace::take_with_place(tab_id, id_of) -> Option<(Tab, TabPlace)>` removes the tab named
  `tab_id` (by `TabViewer::id`, through `id_of`) from its dock, as a close would: an emptied group
  goes, an emptied zone stays unless its dock says otherwise (`DockRules::keep_when_empty`,
  `DockSpace::collapse_empty`).
- `DockSpace::restore_at(tab, place, id_of) -> Result<String, Tab>` puts it back and returns the
  dock it went to, or gives the tab back when the space has no dock left.

`TabPlace` is opaque: the dock, the tabs next to the tab in its group, and its index there; with
the `serde` feature it serialises, and it outlives a layout save and load, since it names tabs by
their id and the dock by its name. `restore_at` puts the tab back as a float's close does
(`ClosePolicy::ReturnToOrigin`), from the same code:

1. In its dock, after the tab that was before it, else before the tab that was after it, in
   whichever of the dock's groups that tab is now: the group comes back through the neighbours.
2. When neither neighbour is in the dock (the tab was alone in its group, or its whole group
   went), in the dock's focused (or first) group, at its old index; in an empty dock, as its only
   tab.
3. When its dock is gone: for a tab taken from a float, where it was before it floated (the
   float's origin, which the place carries); else the first dock no float shows.

The restored tab is its group's active tab. A tab taken from a float and put back in it keeps its
origin, for the float's close.

Hide and show, with the application's own state (`T` already carries what the tab shows):

```rust,ignore
struct App {
    space: DockSpace<Tab>,
    /// Hidden tabs, with where they were.
    hidden: Vec<(Tab, TabPlace)>,
}

impl App {
    fn hide(&mut self, id: egui::Id, viewer: &mut Viewer) {
        if let Some(taken) = self.space.take_with_place(id, |tab| viewer.id(tab)) {
            self.hidden.push(taken);
        }
    }

    fn show(&mut self, id: egui::Id, viewer: &mut Viewer) {
        if let Some(i) = self.hidden.iter().position(|(_, place)| place.tab() == id) {
            let (tab, place) = self.hidden.remove(i);
            if let Err(tab) = self.space.restore_at(tab, place, |tab| viewer.id(tab)) {
                // No dock left: put it wherever the application wants.
                self.space.add_dock("main", DockRules::default(), vec![tab]);
            }
        }
    }
}
```

`hidden` saves with the application's state when it saves it (`TabPlace` is `Serialize`).

### The ghost of a drag

A dragged tab or group leaves its bar and shows a ghost under the pointer: a translucent frame
with a shadow, a tab bar and a hint of the body. A single tab's ghost is a one-tab group's, painted
by `DockPainter::dragged_tab`, whose default paints it as `DockPainter::dragged_group` paints a
group's; both get a `DraggedGroupPaint`. The ghost takes its leaf's size, kept within 160 × 80 and
320 × 200 points, and a group's tabs that do not fit its bar are summed up as "+N".

The application may preview the content under the ghost's tab bar, through two `TabViewer`
methods, for a single tab and for a group (whose active tab is shown):

- `ghost_size(tab, default)` gives the ghost's size, kept within the same limits; `default` is
  the leaf's size, as egui_stack picks it.
- `ghost_body(ui, tab, rect)` paints the body: `ui` covers `rect`, below the tab bar, on the
  ghost's layer, clipped to it and as translucent as the frame. It returns whether it painted;
  the painter then leaves its own body hint out (`DraggedGroupPaint::body_painted`), and the
  application's content goes over the frame the painter drew.

By default neither changes anything. In a space, the ghost is painted by the window under the
pointer, the drag's own window or another: sizes are in points, so the ghost keeps its size from
a screen of one scale to another, and the window the drag began in paints nothing of it (nor the
tab, out of its bar) while the pointer is over another window.

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
