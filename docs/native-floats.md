# Native floating docks (design note)

Status: steps 2 (the model) and 3 (floats drawn, close policy, events, bus points) built;
steps 4 and 5 to come. Tracks [#105](https://github.com/quazardous/egui_stack/issues/105).

Docks in native OS windows (egui viewports) that stay part of one
logical workspace: the same layout tree, the same panel ids, groups,
constraints and persistence. A native window hosts another dock
surface; it is not a second docking system.

## Today

egui_stack draws the main window's areas (`DockSpace` on `LayoutArea`).
The floats of [layout-and-dock.md](layout-and-dock.md) (steps 6–8) are
not built, and the legacy `Surface::Window` / `WindowState` are
`egui::Window`s inside one viewport. Applications that need native
windows (egui-pop, and Helios through it) keep the detached windows,
their places and the drag across windows beside egui_stack's tree. This
note moves that logic into egui_stack so that one implementation serves
every application.

## Model

A floating root is a `layout::Float` whose content is a named dock of
the same `DockSpace`:

```rust
pub struct Float {
    key: String,
    content: Content,      // Dock(name)
    host: FloatHost,       // InWindow | Native: the same float, either host
    place: Place,
    z: u32,
    minimized: bool,
}

pub struct Place {
    screen: Option<String>, // the monitor's name
    rect: Rect,             // points, screen-local
    scale: f32,             // the screen's scale when saved
    maximized: bool,
    fullscreen: bool,
}
```

- `Layout` keeps the places of closed floats, so a float reopens where
  it was.
- `DockSpace` keeps each tab's origin (its dock and neighbours), so a
  closed window can return its tabs.
- The serialised `Layout` gets a new version; old `WindowState` windows
  migrate to `Native` floats.
- Floating windows hold tabs only at first. Splits inside them come
  later.

## The application hosts the windows

egui_stack never creates a window or a rendering surface. The
application implements a host:

```rust
pub trait ViewportHost {
    fn viewport_id(&self, key: &str) -> ViewportId;
    fn builder(&mut self, float: &Float) -> ViewportBuilder;
    fn show(&mut self, ctx: &Context, id: ViewportId, b: ViewportBuilder,
            body: &mut dyn FnMut(&mut Ui));
    fn screens(&self) -> Vec<Screen>;                     // name, physical rect, scale
    fn outer_rect(&self, id: ViewportId) -> Option<Rect>; // None: positions hidden (Wayland)
}
```

Viewport ids are the application's and stay stable, so anything it
pairs with a window (a second wgpu surface, display settings) keeps
working. A 3D view in a floating window is drawn by the application in
`TabViewer::ui`, as anywhere else.

## Dragging across windows

One drag session per `DockSpace`:

- the payload is a tab or a whole group (a group moves whole or not at
  all);
- the pointer is tracked in physical desktop pixels and hit-tested
  against each viewport's docks of the last frame;
- the veto is unchanged (`dock_drop_allowed`, or the `stack.dock.drop`
  request with a bus);
- Escape cancels; a target window closed during the drag cancels;
- releasing over nothing makes a new native float at the pointer; if
  that is refused, the tab goes back;
- a move is applied once, in the next pass: nothing is lost or
  duplicated.

Where the system hides window positions (Wayland), every window that
could take the drop shows an outline, and the first viewport that sees
the pointer over a target shortly after the release takes the drop.
egui_stack never invents a position.

## Closing a window

`ClosePolicy::{ReturnToOrigin, Into(dock), CloseTabs, Keep}`, with
`ReturnToOrigin` as the default, asked through a callback or, with a
bus, the `stack.window.close` request. Window life is reported as polled
events (`DockSpace::take_float_events()`: detached, closed, moved,
repaired), apart from layout changes.

## Screens and DPI

- `Place::repair(&[Screen])` is pure: the title bar must be on some
  screen, and the window is shrunk to fit.
- A window saved on a screen that is gone moves onto an available one
  (`ScreenGone::MoveOnto`, the default) or docks back
  (`ScreenGone::DockBack`).
- Places are saved as points with their scale and re-projected on load;
  global coordinates are physical pixels, converted per viewport, so
  screens with different scales neither overlap nor leave gaps.

## Without and with a bus

The core has no bus: callbacks for decisions, polled events for the
rest. The optional `bus` feature (bus-contract) declares egui_stack's
points under its own prefix, `stack.`; given a bus, the requests replace
the callbacks.

| Point | Kind | Carries |
|---|---|---|
| `stack.dock.drop` | request, fail-closed | the tab or group, source and target docks and floats, the destination → allow or a reason |
| `stack.layout.changed` | event | what changed (resized, folded, unfolded, moved, detached …), zones, docks |
| `stack.window.detached` | event | the float, its dock, tabs, viewport, place |
| `stack.window.close` | request, fails open to the default | the float, its tabs, their origin → a `ClosePolicy` |
| `stack.window.closed` | event | the float, the policy, where the tabs went |
| `stack.window.repaired` | event | the float, the screen it left, the screen it is on |

`stack.dock.drop` is asked when the destination changes and again at
release, never once per frame. Payloads are plain `Send + Sync` values,
never the application's tab type. The bus feature ships together with
the native floats.

## Steps

1. This note.
2. The `Float` / `Place` model, closed places, origins, repair,
   serde version and migration (unit tests: repair, mixed scales, round
   trip, migration).
3. `ViewportHost`, native floats drawn, the close policy, float events,
   and the `bus` feature with its points (tests on a multi-viewport
   harness: open, close returning to origin, keep, screen gone).
4. The drag session across viewports (main to float, float to float,
   float to main, a group, a veto, Escape, release over nothing, target
   closed during the drag, hidden positions, scales 1 and 2).
5. An example with an application-owned wgpu surface, docs, and a
   Windows CI job.

### As built (step 2)

- `layout::Float { key, content, host, place, z, minimized }` and
  `Place { screen, rect, scale, maximized, fullscreen }` as sketched,
  `#[non_exhaustive]`, built with `Float::dock(key, dock, host, place)`,
  `Place::new(rect)` (no screen) or `Place::on(&screen, rect)`.
  `FloatHost::InWindow` is the default host. Nothing draws a float yet.
- `Layout` gains `floats` and `closed`: `open_float` (in front, taking
  the closed place of its key if any), `close_float` (keeps the place),
  `closed_place`, `float`, `float_mut`, `float_of_dock`, `raise_float`,
  `floats_back_to_front`, `repair_floats(&screens, gone)`.
- `Screen { name, rect, scale }`: `rect` in desktop physical pixels.
  `Place::physical(&screen)` and `Place::from_physical(rect, &screens)`
  convert; the offset from the screen's corner is kept in physical
  pixels at the saved scale, the size in points at the screen's scale.
- `Place::repair(&screens, ScreenGone) -> Repaired::{Unchanged,
  Moved(place), DockBack}`, pure: re-projected to its screen's current
  scale, shrunk to fit (then moved inside), moved as little as needed for
  `Place::TITLE_BAR` points of title bar, `Place::GRAB` wide, to lie on
  some screen (a neighbouring screen counts). A place with no screen is
  read on the first screen, at its scale, and never docks back. With no
  screens known, nothing changes.
- `DockSpace::record_origin(path, id_of)` keeps a `TabOrigin { tab,
  dock, before, after }` (tabs named by `TabViewer::id`); `origin`,
  `take_origin`, `origins`. Saved with the space.
- The layout's saved version is 2 (`LAYOUT_VERSION`). A version 1 layout
  loads with no floats; a `Docked` of version 1 runs
  `Docked::windows_to_floats`, which turns each `Surface::Window` into a
  dock `window-N` and a `Native` float keyed alike, at the window's last
  rect with no screen, minimized kept. `DockSpace::windows_to_floats`
  does the space's half for a space saved apart.
- `reconcile` keeps the loaded floats and closed places (also when the
  root changes) and adds the code's floats whose key the loaded layout
  does not know. `Docked::repair` drops floats whose dock is gone
  (`DockedRepair::floats_removed`) and gives no hidden zone to a dock a
  float shows.
- Left for step 3: `ClosePolicy` (the return to origin uses
  `TabOrigin`), drawing, `ViewportHost`, events and the bus points.

### As built (step 3)

- `FloatsArea::new(&mut layout, &mut space)` (or `Docked::floats()`), `.host(&mut dyn
  ViewportHost)`, `.screen_gone(ScreenGone)`, `.style(Style)`, `.show(ctx, viewer)`: called
  once a frame in the main window's pass, after the zones' docks. It draws `InWindow` floats as
  `egui::Window`s over the main window and `Native` floats through the host, back to front, each
  showing its dock with a `DockArea`. Once it has run, the space's floats are on: a tab or a
  group moved out of every dock (released outside them, "Eject", or "Move group" to a new
  window) becomes a float, never a legacy `Surface::Window`, and a legacy window found in the
  space becomes a float too.
- `ViewportHost` as sketched, every method with a default on egui's immediate viewports
  (`EguiViewports`): `viewport_id(key)` (`ViewportId::from_hash_of(("egui_stack.float", key))`),
  `builder(float)`, `show(ctx, id, builder, body)`, `screens()` (none by default: nothing is
  repaired), and `outer_rect(ctx, id)`, which takes the context so its default can read egui's
  `ViewportInfo` (physical pixels; `None` where positions are hidden).
- A native float's builder is made once, from the host's, with the place's position (only on a
  known screen), size and states where the host left them unset, and given again unchanged, so
  egui never moves the window back. Its place follows the window (`outer_rect`, else the size
  only) and `FloatEvent::Moved` is reported once it stops changing. A repair forgets the
  builder, so the window is told to move.
- A tab moved out goes into a dock of its own, `float-N` (rules: kept only while it has tabs),
  which keys the float; its origin is recorded unless it leaves another float. Its place is
  the release point on the desktop (`Place::from_physical` on the host's screens) for a native
  float, else the rect in the main window. `DockSpace::float_out(path, rect, id_of)` and
  `float_out_group` do the same from code. `DockSpace::set_float_host` chooses the host of new
  floats (`InWindow` by default).
- `ClosePolicy::{ReturnToOrigin (default), Into(dock), CloseTabs, Keep}`, asked through
  `TabViewer::close_policy(&FloatClose)` when the user closes a float (its window's close
  button, or the `egui::Window`'s). `Keep` cancels the close (`ViewportCommand::CancelClose`).
  `ReturnToOrigin` puts each tab after the tab it followed (else before the one it preceded,
  else in its dock's focused or first leaf); a tab with no origin, or whose dock is gone, goes
  to the first dock no float shows. `Into` a dock the space has not falls back to the origins.
  A closed float's place is kept (`Layout::closed`).
- `DockSpace::take_float_events() -> Vec<FloatEvent>`: `Detached { key, dock, tabs, host }`,
  `Closed { key, dock, policy, moved }` (`policy` `None` when its dock emptied or went),
  `Moved { key, place }`, `Repaired { key, from, to }` (`to` `None` when docked back).
- Screens: the host's screens are compared with those of the last repair; a change runs
  `Layout::repair_floats`, which now leaves `InWindow` floats alone. Floats docked back
  (`ScreenGone::DockBack`) return their tabs to their origins.
- The docks a float shows are marked; the space's no-drop zones, strips and restore bands are
  the main window's: a dock drawn in a native float's viewport keeps its own drag state (an id
  of its own) and ignores them, and docks drawn in another viewport do not count as "inside a
  dock". A drag from one viewport to another is step 4.
- The `bus` feature (`bus-contract` 0.3.2), `DockSpace::with_bus(Arc<dyn Bus>, key)` /
  `set_bus`: the module `egui_stack::bus` holds the points (`DockDrop`, `LayoutChanged`,
  `WindowDetached`, `WindowClose`, `WindowClosed`, `WindowRepaired`), their plain payloads
  (`DropAsk`, `LayoutChange`, `DetachedInfo`, `CloseAsk`, `ClosedInfo`, `RepairedInfo`), and
  the contract table `POINTS`, declared with `declare_all` (a second space on the same bus
  shares them). `stack.dock.drop` replaces `dock_drop_allowed`; its verdicts are kept per ask
  until the next release, so it is asked when the destination changes, again at release, and
  again when a "Move group" item is chosen. `stack.window.close` starts from the viewer's
  policy; a failure or a refusal gives the default. `stack.layout.changed` carries the
  `LayoutArea` events (through `DockSpace::bind`), tab moves between docks, and float changes.
  Events are also kept for `take_float_events`.
- Deprecated: `DockSpace::windows_dock` and `set_windows_dock` (the legacy windows); they still
  work for a space whose floats are not drawn.
- Not built: groups dragged back into their own leaf (a group returns into its origin dock's
  leaf, not as a leaf of its own), splits inside a float from a drag across windows, a drag
  across viewports (step 4), position snapping and joined groups (layout steps 6 to 8).
- Tests: `tests/native_floats.rs` on a small multi-viewport harness (`tests/support/multi.rs`,
  immediate viewports run as eframe runs them, a close request, positions hidden, windows
  moved), and `tests/bus_points.rs` on a minimal bus (`tests/support/test_bus.rs`).

## Risks

- Wayland: no window positions and no placing at the pointer.
- Windows: per-monitor DPI; a scale change while a window moves must not
  resize it in a loop.
- Mixed DPI: global coordinates have to be physical pixels.
- Two drag sessions (tabs here, other payloads in the application) must
  not both claim one release.
- The cost of a fail-closed veto on the UI thread: `stack.dock.drop`
  needs a short deadline.
