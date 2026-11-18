# Native floating docks (design note)

Status: proposed. Tracks [#105](https://github.com/quazardous/egui_stack/issues/105).

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

## Risks

- Wayland: no window positions and no placing at the pointer.
- Windows: per-monitor DPI; a scale change while a window moves must not
  resize it in a loop.
- Mixed DPI: global coordinates have to be physical pixels.
- Two drag sessions (tabs here, other payloads in the application) must
  not both claim one release.
- The cost of a fail-closed veto on the UI thread: `stack.dock.drop`
  needs a short deadline.
