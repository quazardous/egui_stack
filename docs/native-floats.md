# Native floating docks (design note)

Status: built, steps 2 (the model), 3 (floats drawn, close policy, events, bus points), 4 (the
drag across viewports) and 5 (an application-owned wgpu surface in a float, the Windows CI job).
Tracks [#105](https://github.com/quazardous/egui_stack/issues/105). To use it, see
[Using native floats](#using-native-floats).

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

## Using native floats

1. Keep the window's `Layout` and its `DockSpace` together, as a `Docked`, and choose where new
   floats go: `DockSpace::new().with_float_host(FloatHost::Native)` for native windows
   (`FloatHost::InWindow`, the default, draws them as `egui::Window`s over the main window).
2. Each frame, draw the zones and their docks, bind the space to the layout, then draw the floats
   once, after the docks, in the main window's pass:

   ```rust
   let laid = LayoutArea::new(&mut docked.layout).show_inside(ui, &mut zone_viewer);
   docked.space.bind(&laid);
   docked.floats().host(&mut host).show(ui.ctx(), &mut tab_viewer);
   for event in docked.space.take_float_events() { /* Detached, Closed, Moved, Repaired */ }
   ```

   A tab or a group dragged out of every dock now opens a float; `DockSpace::float_out` does the
   same from code.
3. The host: `EguiViewports` shows the floats in egui's immediate viewports, with every default.
   An application that pairs something with its windows implements `ViewportHost` itself:
   - `viewport_id(key)`: its own id for the float keyed `key`, stable while the float is open;
     key anything to it (a wgpu surface, a camera, display settings);
   - `builder(float)`: the window's title, icon, least size (egui_stack adds the place);
   - `show(ctx, id, builder, body)`: shows the viewport and calls `body` for its content, by
     default `Context::show_viewport_immediate`;
   - `screens()`: the monitors (name, desktop physical rect, scale), so windows left off every
     screen are brought back (`ScreenGone::MoveOnto`, or `DockBack`); none by default;
   - `outer_rect(ctx, id)`: where the window is, `None` where the system hides it (Wayland).
4. Rendering stays the application's: a tab draws what it draws anywhere, in `TabViewer::ui`,
   in whichever viewport it is now (`ui.ctx().viewport_id()`). With eframe's wgpu renderer a
   paint callback (`egui_wgpu::Callback`) works in every viewport, on the device eframe shares
   between them; [`examples/wgpu_float.rs`](../examples/wgpu_float.rs) draws a 3D scene in a
   tab that can be detached.
5. Decisions: `TabViewer::close_policy` says where a closed window's tabs go (back to their
   origin by default), `TabViewer::dock_drop_allowed` vetoes a drop in any window. With the
   `bus` feature, `DockSpace::with_bus` asks them as `stack.dock.drop` and
   `stack.window.close` instead, and tells the events as `stack.*` points.
6. Saving: with the `serde` feature, `Docked` saves the floats, their places (per screen, in
   points with their scale), the closed floats' places and each tab's origin; a layout saved
   before floats loads its legacy windows as native floats.

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

### As built (step 4)

- One drag session per `DockSpace`, kept by the space, whichever viewport its docks are drawn
  in; `DockSpace::dragging() -> Option<SpaceDrag>` reads it: its `payload`
  (`DragPayload::Tab(path)` or `DragPayload::Group(path)`), the `source` viewport, the other
  viewport it is `over`, and whether it is `released` (waiting, see below) or `cancelled`. A
  native float's `DockArea` still keeps its own egui state (its widgets live in its viewport),
  but its drag is the space's.
- The viewport a drag began in, the source, keeps the pointer: the system gives it every pointer
  event of the drag, past its edges too. Each pass it records the pointer in its points and,
  where the system tells where windows are, on the desktop in physical pixels (its
  `inner_rect` × its `pixels_per_point`). Every other viewport showing a dock of the space
  converts the desktop pointer to its own points (its own `inner_rect` and scale, so screens of
  different scales neither overlap nor leave gaps), tests it against its docks as drawn in its
  last pass, shows the usual drop preview (zones, strips, restore bands, no-drop zones and the
  veto all as in one window), and reports to the session what a release would do: a
  destination, a refusal, or nothing under the pointer.
- At its release the source applies the freshest report (made in its last two passes) of the
  viewport under the pointer, once: outside the source, the highest ranked (a native float
  above the main window, a float's z order after); inside the main window, a native float over
  it wins. The destination is asked to the veto again at release (`dock_drop_allowed`, or
  `stack.dock.drop`, whose verdicts are kept per ask until the release, so it is asked when
  the destination changes and at release, never per frame), and checked to still be there.
  Released over no viewport, or over a viewport where no dock is under the pointer, the tab or
  the group opens a new float at the release point (`Place::from_physical` on the host's
  screens), unless the dock's rules, `allowed_in_windows` or the veto refuse it: then it goes
  back. A tear-off from a native float works the same, its origin kept.
- Escape in the source cancels the drag, and so does closing the window under the pointer
  (its float closed by the user, its dock emptied, or docked back): nothing moves at the
  release. A source window that closes ends its drag.
- Where window positions are hidden (Wayland: no `inner_rect`), no other viewport can place the
  pointer: once it leaves the source, each other viewport outlines its docks
  (`DockPainter::window_outline`, `WindowOutlinePaint { rect, refused, .. }`, red where the
  drop would be refused). Released outside the source, the drag waits `RELEASED_PASSES` (12)
  passes of the source; the first viewport that sees its own pointer over one of its docks
  takes it there (asked to the veto again), a refusal ends it, and when nobody takes it, it
  opens a new float with no position (the system places it).
- Origins: a tab that leaves a dock for a float's (by a drag, from any viewport) records
  where it came from, as a drag-out does; a tab that lands in a dock no float shows forgets
  it; from float to float it keeps it. This holds for drags inside one window too.
- A float moved to a screen of another scale is never resized or moved by egui_stack: its
  builder is given unchanged, and its place follows the window (`FloatEvent::Moved` once it
  settles). A drop lands where it is released, on the new scale.
- `TabDestination` and `TabInsert` are `Clone`, `Debug` and `PartialEq` (not `Copy`: semver-checks counts it as a break) now.
- Tests: `tests/drag_across.rs` on the multi-viewport harness (main to float, float to main,
  float to float, a group, a veto, Escape, released over nothing, refused, a tear-off, the
  target closed during the drag, positions hidden with a taker and without, scales 1 and 2,
  a float moved to a screen of scale 2 during a drag) and `tests/bus_points.rs` (the veto
  across windows asked when it changes and at release).
- A float's dock takes the same drop zones as any dock: a tab dropped on a float's edge band
  splits it, as `DockRules::splits` allows.
- Not built: two drag sessions (tabs here, other payloads in the application) sharing one
  release (the application's own drags stay its own), and groups dragged back into their own
  leaf.

### As built (step 5)

- `examples/wgpu_float.rs`, the reporter's "detached 3D viewport": eframe's wgpu renderer, a
  "3D view" tab drawing a spinning triangle with an `egui_wgpu` paint callback (its pipeline,
  uniform buffer and bind group made once, in egui_wgpu's callback resources), and a
  `ViewportHost` of the application's (`Windows`) giving its own viewport ids and pairing a
  tint with each window, kept in egui's memory under the viewport id. The tab draws the same
  in the main window and in a float; dragged back, it renders in the main window again.
  egui_stack needs nothing new for it: rendering stays in `TabViewer::ui`.
- A test (`a_float_lives_in_the_application_s_own_viewport_and_keeps_it`): with a host's own
  ids, the float's window is the host's viewport, its tab is drawn in that viewport, and it
  keeps it when the window moves; closed through it, the tab comes back.
- CI: the `native-floats` job runs the floats tests (`floats`, `native_floats`,
  `drag_across`, `bus_points`) and builds `native_floats` and `wgpu_float` on Windows and
  macOS for pull requests into `next` (the tests job is Linux only there; pushes run every
  test on the three systems). The multi-viewport harness runs egui's viewports headless, with
  no window and no GPU, so no test is platform-specific; Windows is meant as a required
  check, macOS reports only. Real windows (a drag across monitors of different scales on a
  real desktop) are left to manual testing: no CI runner has several monitors.

## Float gestures

In a floating window a drag does what the place it starts says: a tab drags the tab, the group
grip or the group modifier (`FloatsArea::group_drag_modifiers`, Shift by default) drags the
group, and the title bar moves the window. The empty part of a tab bar never moves the window
(an `egui::Window` float is moved by its title bar only, `WindowDrag::TitleBar`).

- **Move handle** (`GroupHandles::MOVE_WINDOW`, on by default, `FloatsArea::group_handles`): at
  the right end of the float's top tab bar (the rightmost if several are on top), apart from the
  group grip, with the move cursor. A window drawn without system decorations can still be
  moved. On a native float the system moves the window (`ViewportCommand::StartDrag`); on a
  float over the main window the `egui::Window` follows the pointer.
- **What a move does** (`FloatsArea::float_move`):
  - `FloatMove::Move`, the default: the window only moves, and never docks.
  - `FloatMove::Dock`, as in Visual Studio: while the window moves, the dock of the main window
    under the pointer is highlighted (its leaf under the pointer, red where refused); released
    there, every tab of the float joins that leaf (or the dock's focused or first leaf, or the
    empty dock), in order, after the veto (`TabViewer::dock_drop_allowed`, or `stack.dock.drop`
    with the bus, asked when the target changes and again at the release; refused, the float
    stays where it was moved). The float closes (`FloatEvent::Closed`, policy `None`) and its
    tabs forget their origins. It needs to know where the window is: on a native float the
    handle moves the window itself (`ViewportCommand::OuterPosition`, from the pointer on the
    desktop), and where positions are hidden (Wayland) the handle falls back to the system's
    move, which never docks. A native window's own title bar is moved by the system, which
    tells nothing of the release: it only moves. Over the main window, both the title bar and
    the handle dock.
- Not built: docking a moved window into another float, or into a split zone (the whole float
  joins a leaf's tabs; dragging its group by the grip offers the zones).
- Tests: `tests/float_gestures.rs` on the multi-viewport harness (the handle's system move and
  cursor, a move never docking by default, `Dock` docking by the handle and by the title bar,
  a refused dock, hidden positions, the handle turned off) and `tests/add_button_place.rs`.

## Risks

- Wayland: no window positions and no placing at the pointer.
- Windows: per-monitor DPI; a scale change while a window moves must not
  resize it in a loop.
- Mixed DPI: global coordinates have to be physical pixels.
- Two drag sessions (tabs here, other payloads in the application) must
  not both claim one release.
- The cost of a fail-closed veto on the UI thread: `stack.dock.drop`
  needs a short deadline.
