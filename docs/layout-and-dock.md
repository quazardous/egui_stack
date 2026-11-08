# Layout and dock: two layers

Status: design note, accepted. Steps 2 to 5 (the model, its solver and serde; `LayoutArea`;
folding; `DockSpace::bind`) are in `src/layout/` and `src/dock_space.rs`; "As built" below lists where it differs from this note. It follows [dock-space.md](dock-space.md), which this
extends: a `DockSpace` shares one drag between several docks, but where each dock is and how big
it is stays the application's. This note moves that part into egui_stack too, as a layer of its
own, and puts the floating windows in it.

## Goals

1. **egui_stack manages a window's whole space**, not only the docks inside the rects an
   application hands it: the side areas' sizes, their least and largest sizes, a centre that keeps
   its least size, folding to a strip, flyouts, the sashes between areas.
2. **Two layers.** A *layout* layer knows zones, sizes and constraints, and nothing about tabs.
   A *dock* layer, today's `DockSpace`, puts its docks in layout zones. Either is usable alone: a
   layout of plain content (a toolbar, a status bar, a viewer) needs no dock, and
   `DockArea::new(&mut DockState)` keeps working with no layout.
3. **Zones may refuse drops.** A zone that holds no dock, or a dock with
   `DockRules::accept_drops(false)` (shipped in 0.25.3), is never a drop target, and a tab released
   there goes back.
4. **Floating windows belong to the layout.** Their geometry, their snapping (by position *and*
   by size) and their groups (windows joined edge to edge, drawn as one frame) are layout
   concerns; their tabs stay the dock's.
5. **A small contract for consumers.** An application such as
   [egui-pop](https://github.com/quazardous/egui-pop) should depend on a short list of items:
   the layout, the dock with its drop rules, the painter with one style value. egui_stack never
   depends on its consumers.
6. **One serialisable value per layer**, and a migration from today's `DockState` and
   `DockSpace` values.

Non-goals: detached OS windows (viewports) as layout members, beyond what `DockSpace` already
plans; animation; a declarative layout language.

## Prior art

| Project | What it does | What we take |
|---|---|---|
| [egui_tiles](https://github.com/rerun-io/egui_tiles) | One tree of tiles (tabs, linear, grid containers) where every pane is a tile; shares per child. | Its tree is tabs and layout in one, which is what this note separates. We keep one tree per layer. |
| [egui_dock](https://github.com/anhosh/egui_dock) (upstream) | One `DockState` per area, floating windows as `egui::Window`s; the application places areas with egui panels. | Today's starting point: the dock layer is its descendant. |
| [Dear ImGui docking](https://github.com/ocornut/imgui/wiki/Docking) | Dock nodes, a central node that keeps room, nodes that refuse docking (`NoDockingOverMe`, `NoDockingSplit`), floating windows that host dock trees. | The central node, per-node drop refusals, a floating window as a host of a dock. |
| [VS Code's grid](https://github.com/microsoft/vscode/tree/main/src/vs/base/browser/ui/grid) | `GridView` of `SplitView`s; each view has `minimumSize`, `maximumSize`, a `LayoutPriority` (low gives way first, high last) and `snap` (a view dragged under its least size snaps hidden). | Priorities instead of proportional shrinking, snap-to-fold on a sash, panel alignment as tree shapes. |
| [JetBrains tool windows](https://www.jetbrains.com/help/idea/tool-windows.html) | Tool windows on stripes around the editor, modes Dock pinned / Dock unpinned (a flyout) / Undock (over the editor) / Float / Window. | Our fold strip, flyout and "lying over" are these modes; the editor never folds. |
| [Blender areas](https://docs.blender.org/manual/en/latest/interface/window_system/areas.html) | The window is fully tiled by areas; areas split and join by dragging a corner; a shared edge resizes both. | Joined edges that resize together; joining as an explicit act. |
| [Rerun's layout](https://rerun.io/docs/concepts/blueprint) | egui side and bottom panels around an egui_tiles viewport, the arrangement kept in a "blueprint". | The split we make: panels around, docks inside, one saved value. |

Window managers (KWin's quick tiling, Windows Snap layouts) and creative applications
(Photoshop, Lightroom and Affinity palettes) inform the floating-window section: magnetic edges,
a guide line while snapping, a modifier to suspend it, stacked palettes moved as one.

## The two-layer model

```
Layout (egui_stack::layout)                 Dock (egui_stack::DockSpace)
  zones: tree of splits and leaves            docks: name -> Tree<Tab>, DockRules
  constraints and state per zone              tabs, splits, grids, drag and drop
  floats: rect, z, group                      windows: name -> Tree<Tab>
  groups: joints between floats
        |                                              |
        +--- a leaf zone holds Content::Dock("left") --+
        +--- a float holds Content::Dock("win-3") -----+
```

The layout lays out rects; the dock fills some of them. The only link is a name: a leaf zone or a
float whose content is `Content::Dock(name)` shows the `DockSpace` dock of that name.

### Layout types

```rust
pub mod layout {
    /// A zone's name: stable, saved, chosen by the application ("left", "centre", "status").
    pub struct ZoneName(pub String);

    /// The whole layout: one serialisable value.
    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
    pub struct Layout {
        root: Zone,
        floats: Vec<Float>,
        groups: Vec<Group>,
        version: u32,
    }

    pub struct Zone {
        pub name: ZoneName,
        pub kind: ZoneKind,
        pub constraints: Constraints,
        pub state: ZoneState,
    }

    pub enum ZoneKind {
        /// Children side by side (`Horizontal`) or one above the other (`Vertical`).
        Split { dir: Dir, children: Vec<Zone> },
        Leaf { content: Content },
    }

    pub enum Content {
        /// The `DockSpace` dock of that name.
        Dock(String),
        /// Anything the application draws: a toolbar, a status bar, a viewer.
        App,
        /// Room left empty.
        Empty,
    }

    /// A size along the parent's axis.
    pub enum Size {
        Points(f32),
        /// A share of the parent zone's length (0.25: a quarter); of the window's content at the root.
        Share(f32),
    }

    pub struct Constraints {
        pub min: Option<Size>,
        pub max: Option<Size>,
        /// The size a zone starts with and goes back to on a sash's double click.
        pub default: Option<Size>,
        /// The zone takes what its siblings leave (the centre). One per split at most.
        pub grow: bool,
        /// Who gives way first when room is short: `Low` before `Normal` before `High`.
        pub priority: Priority,
        /// The least size includes what the content asked for last frame (its header, a toolbar).
        pub fit_content: bool,
        pub fold: Fold,
        /// Whether a dragged item may be dropped here (see "Drop rules").
        pub drops: Drops,
    }

    pub enum Priority { Low, Normal, High }
    pub enum Fold { Never, ToStrip }
    pub enum Drops { Accept, Refuse }

    /// What the user changed: saved.
    pub struct ZoneState {
        /// The size the user gave, kept even while the zone is shown smaller.
        pub size: Option<f32>,
        pub folded: bool,
        /// Unfolded over its neighbour, next to its strip, without taking room.
        pub over: bool,
        pub hidden: bool,
    }

    /// A floating window, in the window's content coordinates.
    pub struct Float {
        pub key: String,
        pub content: Content,
        pub rect: egui::Rect,
        pub minimized: bool,
        pub z: u32,
    }

    /// Floats joined edge to edge: they move together and are drawn as one frame.
    pub struct Group {
        pub joints: Vec<Joint>,
    }

    /// Two floats touching along a segment of one edge each.
    pub struct Joint {
        pub a: String,
        pub b: String,
        /// `a`'s edge that touches `b`'s opposite edge.
        pub side: Side,
    }

    /// Draws a layout: lays out its zones in the `Ui`'s rect, its sashes, strips and flyouts,
    /// then calls the viewer for each zone's content.
    pub struct LayoutArea<'a> { /* layout, id, style, painter */ }

    impl<'a> LayoutArea<'a> {
        pub fn new(layout: &'a mut Layout) -> Self;
        pub fn id(self, id: egui::Id) -> Self;
        pub fn style(self, style: LayoutStyle) -> Self;
        pub fn painter(self, painter: &'a (dyn LayoutPainter + Sync)) -> Self;
        pub fn flyout(self, flyout: Option<Flyout>) -> Self;
        pub fn show_inside(self, ui: &mut egui::Ui, viewer: &mut dyn ZoneViewer) -> Laid;
    }

    /// What the application draws in the layout.
    pub trait ZoneViewer {
        /// A zone's content, in its rect.
        fn ui(&mut self, ui: &mut egui::Ui, zone: &ZoneName);
        /// A folded zone's strip; by default the painter's grip only.
        fn strip(&mut self, ui: &mut egui::Ui, zone: &ZoneName) {}
        /// A float's content.
        fn float(&mut self, ui: &mut egui::Ui, key: &str) {}
        /// A flyout's content.
        fn flyout(&mut self, ui: &mut egui::Ui, flyout: &Flyout) {}
    }

    /// A flyout: one item shown over the zone next to a folded zone's strip.
    pub struct Flyout {
        /// The application's key (a panel), under which its width is kept.
        pub key: String,
        /// The folded zone whose strip it opens from.
        pub zone: ZoneName,
    }

    /// Where everything was drawn this frame: each zone's and float's rect, the strips, the
    /// flyout, and what changed (a zone resized, folded, a float moved or joined).
    pub struct Laid { /* rects by name, events */ }
}
```

The application builds the tree in code (or loads it), and can change constraints every frame:
`layout.constrain("left", |c| c.min = Some(Size::Points(120.0)))`. The tree's shape changes through
methods (`insert`, `remove`, `move_zone`, `set_dir`), which keep each zone's state.

### Dock types, on top

`DockSpace` stays as it is. One function binds it to a layout's drawing:

```rust
impl<Tab> DockSpace<Tab> {
    /// Reads where the layout put its zones and floats: each `Content::Dock` names a dock,
    /// each zone or float with `Drops::Refuse`, or with no dock, becomes a no-drop zone this frame.
    pub fn bind(&mut self, laid: &Laid);
}
```

and a viewer helper that draws a zone's dock:

```rust
struct Viewer<'a> { space: &'a mut DockSpace<Panel>, tabs: &'a mut PanelViewer }

impl ZoneViewer for Viewer<'_> {
    fn ui(&mut self, ui: &mut Ui, zone: &ZoneName) {
        match zone.0.as_str() {
            "toolbar" => toolbar(ui),
            dock => { DockArea::dock(self.space, dock).show_inside(ui, self.tabs); }
        }
    }
}
```

Because egui_stack resolves a drag one frame late (dock-space.md), `bind` may run after the layout
was drawn: the zones' rects of this frame serve next frame's drag, as the docks' own rects already
do.

## The layout algorithm

The layout is solved one split at a time, from the root down, along the split's axis. Across the
axis every child takes the split's whole breadth.

**Inputs per child**, for a split of length `L`:

- `min`, `max` resolved to points (`Share` against `L`; at the root, the window's content);
  `fit_content` raises `min` to the size the content asked for last frame;
- its wanted size: the user's `state.size`, else `default`, else an even share;
- whether it is folded (it takes its strip's breadth, from the style), lying over (its strip's
  breadth in the flow, its body drawn over the neighbour) or hidden (nothing);
- `grow` (the centre) and `priority`.

**Steps:**

1. Folded, lying-over and hidden children take their fixed breadth (strip or zero).
2. Every other child that does not grow takes its wanted size, clamped to `[min, max]`.
3. The growing child takes what is left. If that is under its own `min`, the others **give way**,
   lowest priority first, then the nearest to the growing child; each gives way down to its
   `min`, never in proportion. This is the rule `sections::plan` already applies to section
   heights, and the solver shares that code. A centre's least size thus wins over the sides'
   wanted sizes and their `max`, never over their `min`.
4. If even the least sizes do not fit, zones with `Fold::ToStrip` and an opt-in `auto_fold` fold,
   lowest priority first (as VS Code hides a view), and come back when the room does; the rest is
   cut at the window's edge.
5. With no growing child, the last child takes what is left (or gives back what is missing),
   within its bounds; a split of equals (a row of docks) keeps shares instead.

The user's `state.size` is **not** rewritten when a zone is shown smaller: when the window grows
again, the zone gets its size back. Only a sash drag writes it (again as `sections` does).

**Sashes.** One sash between two neighbouring zones that are both shown and not folded. Dragging it
moves the boundary; the zones on the shrinking side give way nearest first, down to their `min`,
and the growing side up to its `max`; the centre's least size bounds the drag. Keyboard: the sash
takes focus, arrows move it by a row, Enter or a double click goes back to the default sizes.
Accessibility: each sash is a named splitter ("Resize left area").

**Sash snapping.** Within the style's magnet distance (default 8 points), a dragged sash catches:

- the zone's `default` size;
- the position of another sash on a parallel line (a bottom area's edge lines up with a side area's
  edge);
- the fold threshold: dragged under `min` by more than half of it, a `Fold::ToStrip` zone folds on
  release; dragged out of a strip, it unfolds.

A guide line (painter's `snap_guide`) shows what it caught. Holding Alt (Option on macOS) suspends
snapping during a drag.

**Folding.** A folded zone keeps a strip in the flow, so its neighbours keep their sizes (a
terminal in the centre is not resized). The strip is drawn by the painter (`strip`, its grip) and
by the viewer (`ZoneViewer::strip`, the application's icons); a click on the grip toggles. A side
zone may also lie **over** its neighbour: unfolded, next to its strip, without taking room, with
the style's shadow. The growing zone never folds.

**Flyouts.** An application asks for one flyout at a time: an item of a folded zone (one panel of
its dock), shown over the neighbouring zone, next to the strip, without moving the layout. The
layout draws its frame and its free edge, which resizes it; the width is kept per flyout key in the
`Layout` value. Escape, a click outside, or the strip's grip closes it; the application decides
what it shows and when to open it. Unfolding the zone closes it.

## Docks in zones

- A leaf zone with `Content::Dock(name)` shows that `DockSpace` dock in its rect; the zone's
  constraints are the dock's (a dock asks for no size of its own). A zone with `fit_content`
  measures its tab bar.
- One dock per zone, one zone per dock. A zone without a dock is plain content.
- A folded zone hides its dock; its tabs stay in the `DockSpace`. The strip may list them (the
  viewer asks the space for the dock's tabs).
- Moving a dock to another zone is renaming the content, not moving tabs.
- An empty dock zone with `keep_when_empty` stays as a drop target; without it, the dock layer
  hides the zone (`state.hidden`) rather than removing it, so it comes back where it was.
- The floating windows' tab trees stay surfaces of the `DockSpace`; their geometry moves to the
  layout's floats (below).

## Drop rules

A drag is resolved by the dock layer as today, with one more source of refusals:

| Where the tab is released | Result |
|---|---|
| A dock that accepts drops | Today's zones: centre, edge bands, a slot in a tab bar. |
| A dock with `DockRules::accept_drops(false)` | Refused: goes back, no preview. |
| A zone with `Drops::Refuse`, or with no dock | A no-drop zone: goes back, no preview. |
| A zone with `Content::Empty` and `Drops::Accept` | A new floating window there (as outside every dock today). |
| A folded zone's strip, its dock accepting drops | Appended to the folded dock (see open questions). |
| A float holding a dock | Its dock's zones, as any dock. |
| A float with plain content | Refused, as a zone with no dock. |
| Outside every zone and float | A new floating window. |

`bind` turns the layout's refusals into `DockSpace::no_drop_zone` rects each frame, so the dock's
existing test covers them. `TabViewer::drop_allowed` and `dock_drop_allowed` stay the vetoes, with
their reason on the preview. The layout layer itself never resolves a tab drop: `Drops` is data
the dock layer reads.

## Floating windows

Today a floating window is an `egui::Window` per window surface of the `DockState`, its geometry in
`WindowState`. egui draws its frame, and it snaps to nothing.

In the layout, a **float** is a rect with a key, a z-order, a minimized flag and a content
(`Content::Dock` for a dock window, `Content::App` for an application's palette). The layout draws
its frame through the painter (no `egui::Window` frame), so that a group can be painted as one.
The dock window's body is the dock of the same name.

### Snapping by position

While a float moves, each of its four edges looks for a target within the magnet distance:

- the window content's edges, and the layout zones' edges (a sash, the centre's border);
- the other floats' edges, outer to outer (side by side) and same to same (aligned).

x and y snap independently; the nearest target wins on each axis. A guide line shows each catch.
Alt suspends it.

### Snapping by size

While a float is resized, the edge being dragged catches:

- the same targets as a move (an edge of another float, of a zone, of the window);
- **a neighbour's size**: the float's width equals a float's width, or its height a float's height,
  preferring floats it is aligned with or touching. The guide shows the two equal spans with a
  size mark on each, as design tools do;
- never under the content's least size (a dock's tab bar, a palette's declared minimum).

Two floats side by side can thus share an exact height, and a float resized near another catches
on its edge.

### Joined groups

A move that ends with an edge snapped against another float's opposite edge, touching along at
least the style's `join_overlap` (default: 24 points, or the whole shorter edge), **joins** them:
a `Joint` is recorded. Joints are state, saved, not recomputed each frame, so a group never
flickers in and out. A group is the connected set of floats linked by joints.

- **Moving.** Dragging a member's title bar moves the whole group. With Ctrl (Cmd on macOS) the
  member is dragged alone and leaves the group; "Detach" in its menu does the same.
- **Shared edge.** The segment two members share is a sash: dragging it resizes both, as between
  docked zones, bounded by each one's least size. An outer edge resizes only its own float;
  resizing past the joint breaks it.
- **Breaking.** A joint goes when its floats no longer touch (a member closed, moved alone, or
  resized away). The group splits into its connected parts.
- **Focus and z-order.** Clicking a member raises the whole group, the member on top within it;
  the keyboard focus goes to the member.
- **Saved** with the layout: the floats' rects and the joints.

The alternative, a floating split tree (a float holding a `Zone`, as Blender's areas or ImGui's
floating dock nodes), resizes and paints trivially but cannot express floats that touch along
part of an edge, or L shapes. Joints cover both; a group whose floats tile a rectangle could later
be offered "make it one window" (a floating split tree).

### The merged frame

A group is painted as one assembled piece, through a painter call with the group's outline:

- one shadow, around the union of the members' rects, drawn once below all of them;
- one border along the outline; on a shared segment, a single hairline (`joint_stroke`) instead of
  two borders; only the shared segment is merged when edges overlap in part;
- corners rounded at the outline's convex corners only; corners on a joined side are square, and
  concave corners (an L shape) are square;
- each member keeps its own title or tab bar and its own buttons (close, minimise), restyled with
  square corners on its joined sides; there is no merged title bar for now;
- a minimized member keeps its bar in the group; the group's outline follows.

New style and painter items, so a theme styles all of it:

```rust
pub struct WindowGroupStyle {
    pub joint_stroke: egui::Stroke,
    pub joined_corner_radius: f32,
    pub shadow: egui::Shadow,
    pub magnet: f32,
    pub join_overlap: f32,
}

pub trait LayoutPainter {
    fn sash(&self, painter: &egui::Painter, sash: &SashPaint<'_>);
    fn strip(&self, painter: &egui::Painter, strip: &StripPaint<'_>);
    fn snap_guide(&self, painter: &egui::Painter, guide: &SnapGuidePaint<'_>);
    /// A float or a group of joined floats: shadow, outline, joints.
    fn float_frame(&self, painter: &egui::Painter, frame: &FloatFramePaint<'_>);
    fn flyout_frame(&self, painter: &egui::Painter, frame: &FlyoutPaint<'_>);
}
```

`DockPainter` gains `LayoutPainter` as a supertrait with default methods drawn from `Style`, so
one painter value paints both layers and existing painters keep compiling.

## Serialisation and migration

With the `serde` feature:

- `Layout` serialises as one value: the zone tree with each zone's state, the floats, the
  joints, the flyout widths, and a `version`. Constraints are not saved: after loading,
  `Layout::reconcile(&code)` takes them from the tree the code builds, so a saved layout never
  overrides a newer least size, and adds or drops the zones the code added or removed.
- `DockSpace` serialises as today (its `DockState`, docks, rules).
- `Docked<Tab> { layout: Layout, space: DockSpace<Tab> }`, an optional pair, serialises both in
  one value for an application that keeps them together. `Docked::repair` drops floats and zones
  whose dock is gone and docks that no zone names (they become hidden zones).

Migration:

| From | To |
|---|---|
| A `DockState` (one `DockArea`) | `Layout::single("main")` (one leaf, `Content::Dock("main")`) and `DockSpace::from_states([("main", DockRules::default(), state)])`. |
| A `DockSpace` placed by egui panels | `Layout::from_space(&space, shape)`: the application gives the tree shape (which dock is left, which grows); each dock becomes a leaf. |
| `WindowState` of each window surface | A `Float` with the window's last `screen_rect` and `minimized`, keyed by the window's name. |
| `sections::Layout` | Unchanged: `Sections` stays a widget; its plan is the layout's 1D solver. |

`DockArea::new(&mut DockState)` with no layout keeps drawing windows as `egui::Window`s, from
`WindowState`: nothing changes for it. Once a `DockSpace` is bound to a layout, the layout owns
the windows' geometry and `WindowState`'s position and size are ignored; they are deprecated once
every path is on the layout.

## As built (step 2)

Where the `layout` module differs from the sketch above:

- `Layout` holds the zone tree only (`root`); `floats`, `groups` and the flyout widths come with
  their steps (4, 6, 8). The `version` is not a field: it is written on save
  (`LAYOUT_VERSION`) and a later version fails to load (`VersionError`).
- `Constraints` gains `auto_fold: bool`, the opt-in of step 4 of the algorithm (with
  `Fold::ToStrip`). A zone folded by the solver is reported as `Shown::AutoFolded` and its
  `state.folded` is not written, so it comes back with the room.
- Giving way is ordered by priority, then nearest to the growing child (else the last open child)
  first, as sections give way nearest the boundary; the solver keeps no memory of the last sash
  dragged. The shared rule is `sections::plan::give_in_turn`.
- A growing child's `max` holds only while the other children can take the surplus up to their
  `max`; what is left still goes to it. With no growing child, it is left empty after the last.
- The solver is `Layout::solve(rect, &Solve) -> Laid`, pure. `Solve` carries the strip breadth
  and the content sizes asked last frame (for `fit_content`). `Laid` lists `LaidZone { name, rect,
  shown, strip }`, parents first: a folded zone's `rect` is its strip; a zone lying over has its
  strip in the flow and its body, towards the growing child, as `rect`; zones under a folded or
  hidden zone are not laid. Events come with `LayoutArea` (step 3).
- `reconcile` keeps the loaded tree's shape: a zone the code moved stays where it was saved, a
  zone only the code has is added after the same sibling, the kept zones under a split the code
  dropped take its place, and a leaf turned split (or back) takes the code's kind. With another
  root, the code's layout replaces the loaded one.

## As built (step 3)

- `LayoutArea::new(&mut layout).id(..).style(..).painter(..).show_inside(ui, viewer)` returns
  the solver's `Laid` with `events` (`LayoutEvent::Resized`, `Reset`); each `LaidZone` also
  carries its solved `least` and `most`, its `content` and its `drops`.
- A sash resizes one zone: the one on its side away from the zone that takes the rest (the
  growing one, else the last open one), bounded by that zone's `min` and `max` and by what the
  taker has above its own `min`. The neighbours further away do not give way during a drag: the
  solver does that when the window shrinks. Snapping and its guide (`snap_guide`), and Alt, are
  not built yet.
- `LayoutPainter` is a trait of its own with defaults drawn from `LayoutStyle`
  (`DefaultLayoutPainter`), not a supertrait of `DockPainter`: that would break every painter
  outside the crate. A host implements both on one type.
- `LayoutStyle` is its own value (`LayoutStyle::from_egui`), not a field of the dock's `Style`.
- Each leaf zone is a pane named after the zone; each sash a splitter named "Resize <zone>",
  whose value is the zone's size, with increment, decrement and set-value actions.

## As built (step 4)

- A strip's grip is a button ("Unfold left" for a folded zone, "Fold left" for one lying
  over) at the strip's start; the rest of the strip is `ZoneViewer::strip`'s. An auto-folded
  zone's grip does nothing: the room decides.
- A zone lying over (`ZoneState::over`) is drawn in a foreground `egui::Area` over its
  neighbour, framed by `LayoutPainter::flyout_frame`, and folds back on Escape or a press
  outside it and its strip, as JetBrains' unpinned tool windows do. It is opened from code
  (`Layout::set_over`), for example from the application's strip icons.
- The flyout is drawn the same way, over the parent split towards its far side, its breadth
  taken from `Layout::flyouts`, else the zone's size, else `LayoutStyle::flyout_size`. It asks to
  close through `LayoutEvent::FlyoutClosed`; the application owns the `Option<Flyout>`. Its free
  edge is a splitter ("Resize <key>").
- Dragging a strip to unfold, and the guide line of the fold threshold, are not built yet.

## As built (step 5)

- `DockSpace::bind(&mut self, laid: &Laid)` as sketched: `LaidZone` carries each leaf's content
  and drop rule, so the `Laid` alone is enough. A folded zone's strip is a no-drop zone for now
  (open question 7 is not settled); a zone lying over that refuses drops refuses over its body.
- Not built yet in this step: `Docked` with its repair, `Layout::from_space`, the demo on the
  layout, and the strips as drop targets.

## The minimal contract

What a consumer such as egui-pop should depend on, and nothing else:

| Part | egui_stack items | What the consumer does with them |
|---|---|---|
| Layout | `layout::{Layout, Zone, ZoneKind, Content, Constraints, Size, Priority, Fold, Drops, ZoneState, Float, Flyout, LayoutArea, ZoneViewer, Laid}` | Builds its shell's tree, sets constraints from its settings, draws zone contents, reads rects and events. |
| Dock | `DockSpace`, `DockArea::dock`, `TabViewer`, the space's editing API (`add_dock`, `dock_mut`, `Tree` with its node types for arrangement), `TabPath` | Holds the panels' tabs, arranges groups, draws each dock in its zone. |
| Drop rules | `DockRules` (`accept_drops`, `windows`, `splits`, `keep_when_empty`), `Drops`, `DockSpace::no_drop_zone(s)`, `TabViewer::drop_allowed`, `dock_drop_allowed` | Refuses drops per area and per panel; vetoes moves with a reason. |
| Painter and style | `DockPainter` (with `LayoutPainter`) and its `*Paint` structs, `Style` (one value, including `LayoutStyle` and `WindowGroupStyle`) | Paints everything with its own kit; maps its theme into one `Style`. |

Widgets outside the contract that a consumer may still use, behind the same adapter: `sections`
(stacked sections), `toolbar` (tool rows), `Labels`.

## egui-pop's adapter plan

### Today's coupling surface

egui-pop (main, on egui_stack 0.25.2) uses egui_stack in four places. Items marked **public**
appear in egui-pop's own public API today.

| Where | egui_stack items | Public? | Behind the adapter |
|---|---|---|---|
| `egui_pop_core::sections` | re-exports `sections::{Section, Stack, Laid, Remembered, laid_out, row_height}`, `sections::plan` and `plan::DragMode` | **public** | egui-pop-owned `Section`, `Stack`, `Laid`, `Remembered` wrappers (same methods, egui-pop types), `laid_out` and `row_height` as egui-pop functions; the re-exports are kept one release with `#[deprecated]` pointing at the wrappers, then removed. `plan` is not re-exported any more (an application needing it depends on egui_stack itself). |
| `egui_pop_core::sections` | `sections::{Sections, Layout}` (conversions, storage), `Labels` | no | Inside the adapter: egui-pop's `Layout` stays its own type, converted there. |
| `egui_pop_core::kit` (`kit/dock.rs`) | `impl DockPainter for Kit`; `DropButtonPaint`, `DropPaint`, `DropTargetPaint`, `PartState`, `SectionBandPaint`, `SectionBoundaryPaint`, `SeparatorAxis`, `SeparatorPaint`, `Split`, `TabBarPaint`, `TabBarTailPaint`, `TabMeasure`, `TabPaint`, `ToolItemPaint`, `ToolMorePaint`, `ToolSeparatorPaint`, `toolbar::ToolKind` | **public** (a trait impl on a public type) | The impl moves to an adapter newtype, `KitPainter<'a>(&'a Kit)`, so `Kit`'s public API names no egui_stack trait. Later it implements `LayoutPainter` too. |
| `egui_pop_api::workbench` | `DockArea`, `DockState`, `Tree`, `Node`, `LinearNode`, `LinearDir`, `GridNode`, `GridLayout`, `NodeIndex`, `TabIndex`, `Split`, `TabViewer`, `TabPath`, `AllowedSplits`, `DropZone::at`, `Style::from_egui`, `OverlayType`, the `*Paint` types (its `Frameless` painter) | no (the workbench's state is crate-private) | Inside the adapter: the conversions between egui-pop's layout document (`model::layout::Node`) and dock trees, the tab viewer, the dock style, the frameless painter. |
| `egui_pop_api::workbench::WindowDrop` | field `zone: egui_stack::DropZone` | **public** | An egui-pop enum (`Joins`, `Splits(side)`, `Tab(index)`), converted in the adapter; its test follows. |
| `egui_pop_api::toolbars` | `toolbar::{ToolRow, ToolDisplay}`, `Labels` | no | Inside the adapter. |
| `egui_pop_model::sections` | none (a deprecated copy of the plan, already due for removal) | **public**, deprecated | Removed as already announced. |
| Cargo | `egui_stack` in the workspace, used by `egui-pop-core` (with `serde`) and `egui-pop-api` | | Unchanged dependency, one module using it per crate (below). |

The `egui_pop` facade re-exports no egui_stack item today, and must not start.

### The adapter

- **One module**, `egui_pop_core::stack`: every `use egui_stack` of egui-pop lives there, and in
  nothing else. It lives in egui-pop-core, the lowest crate that needs egui_stack (kit, sections).
  egui-pop-api's workbench and toolbars reach egui_stack only through it; it is
  `#[doc(hidden)] pub`, documented as internal and outside semver.
- **Its surface is egui-pop-shaped**: functions and types taking egui-pop's own (`PanelId`,
  `Area`, `ShellSize`, `model::layout::Node`, `Kit`), so that a change of egui_stack's API is a
  change of this module only. A CI check (a `grep` in `scripts/`) fails when `egui_stack::`
  appears outside it.
- **Contract only**: the module uses the items of the contract table and the two widgets, and
  nothing else; a new need is a new contract item, decided upstream first.

### The shell on the layout

The workbench's `egui::Panel`s (activity bar, strips, side and bottom areas, central panel) go.
The adapter builds a `Layout`:

```
root (Horizontal)
  activity_bar   App, fixed size, Drops::Refuse
  start          Dock("start"), Fold::ToStrip, priority Normal
  middle (Vertical)
    centre       Dock("centre"), grow
    bottom       Dock("bottom"), Fold::ToStrip
  end            Dock("end"), Fold::ToStrip
```

Panel alignment (the bottom area under the centre only, or across the whole width) is a choice
of tree shape, rebuilt by the adapter when the setting changes; zones keep their state by name.
Sides swapped (the side bar on the right) is the same tree, children reversed.

| egui-pop today | On the layout |
|---|---|
| `AppBuilder::shell_panel_min_size(panel, size)` | The `min` of the zone holding the panel: the largest of its panels' least sizes, as today. |
| `AppBuilder::shell_panel_max_size(panel, size)` | The zone's `max`: the smallest of its panels' set maxima. |
| `AppBuilder::centre_min_size(size)` | The centre zone's `min`, the centre being the zone that grows. |
| `ShellSize::{Points, Share}` | `layout::Size::{Points, Share}`, converted in the adapter. |
| Hard-coded least width 48, defaults 240 (sides) and 200 (bottom) | Default constraints the adapter sets, overridable. |
| `PanelSpec::min_size`, `default_size` | The zone's `min` and `default`, folded over its panels. |
| Folded areas, areas lying over the centre (`layout.folded`, `layout.over`) | `ZoneState::folded`, `ZoneState::over`. |
| The strip and its icons | The layout's strip; the icons from `ZoneViewer::strip`. |
| `workbench.flyout`, flyout widths per panel | `LayoutArea::flyout(Some(Flyout { key: panel, zone }))`; widths in the layout value. |
| Zen mode | Every zone but the centre hidden. |
| Detached panels' windows, palettes | Floats (`Content::Dock`, `Content::App`): one snapping and grouping model for both. |

egui-pop's saved layout document keeps its format and stays the source of truth: the adapter
builds the egui_stack values from it at start and writes the user's changes (sizes, folds, flyout
widths, floats) back into it. The workbench's steps (`workbench.fold`, `workbench.flyout`, …) are
asked before the adapter applies a change the layout reports in `Laid`, so a plugin can still veto
them.

## Phased plan

Each step is one pull request; the egui_stack steps ship in minor releases, each usable alone.

**egui_stack**

| # | Pull request | Size |
|---|---|---|
| 1 | This design note. | S |
| 2 | `layout` module, model only: types, the 1D solver (shared with `sections::plan`), serde, unit tests; nothing drawn. | M |
| 3 | `LayoutArea`: zones drawn, sashes (drag, keys, accessibility, snapping, double click), `fit_content`, `LayoutPainter` with defaults from `Style`; an example. | L |
| 4 | Folding to a strip, lying over, flyouts. | M |
| 5 | The dock on the layout: `DockSpace::bind`, `Drops` as no-drop zones, folded strips as targets, `Docked` with its repair, migrations; the demo moves onto it. | M |
| 6 | Floats in the layout: the frame painted by egui_stack, `WindowState` migrated, position snapping with guides. | M |
| 7 | Size snapping: edges and neighbours' sizes, guides with size marks. | M |
| 8 | Joined groups: joints, group move and detach, shared-edge sashes, z-order, the merged frame (`float_frame`, `WindowGroupStyle`). | L |
| 9 | Deprecations: `WindowState`'s geometry setters under a layout; docs. | S |

**egui-pop**

| # | Pull request | Size |
|---|---|---|
| A | The adapter module and the CI check; every use moved behind it with no behaviour change; `WindowDrop::zone` as egui-pop's own enum; `KitPainter`; the `sections` re-exports deprecated in favour of wrappers. Can start now, on 0.25. | M |
| B | The workbench on `DockSpace` (one space for the areas, in progress), if not done before A. | M |
| C | The shell on the layout (after egui_stack 5): the `egui::Panel`s go, `AppBuilder`'s sizes become constraints, strips and flyouts from the layout, the saved document unchanged. | L |
| D | Detached panels' windows and palettes as floats (after egui_stack 8): one snapping and grouping model, the kit's tokens for joints and group shadow. | M |
| E | The deprecated re-exports removed, in the next release after A. | S |

## Open questions for the owner

1. **Module or crate.** The layout as a module of egui_stack (`egui_stack::layout`, proposed), or
   a sibling crate in the same repository (`egui_stack_layout`) that egui_stack depends on?
2. **What a saved layout holds.** Proposed: the tree and the user's state (sizes, folds, floats,
   joints, flyout widths), with constraints always re-asserted from code. Or are constraints
   saved and authoritative?
3. **Short of room.** When even the least sizes do not fit: zones fold automatically, lowest
   priority first, and come back with the room (proposed, opt-in per zone), or everything is cut
   at the window's edge?
4. **Groups.** Joints between free floats (proposed: partial overlaps and L shapes), or floating
   split trees (simpler, rectangles only)?
5. **Modifiers.** Alt (Option) to suspend snapping, Ctrl (Cmd) to drag one member out of a group:
   those, or others?
6. **Title bars in a group.** Each member keeps its own bar (proposed), or a row of joined floats
   gets one merged bar?
7. **A tab dropped on a folded strip.** Appended to the folded dock (proposed), or the zone
   unfolds after a short hover, as JetBrains' stripes do?
8. **Detached OS windows.** Kept out of snapping and groups for now (proposed)?
9. **The adapter's place.** In egui-pop-core as a hidden public module (proposed), or egui-pop-core
   drops egui_stack and the adapter lives in egui-pop-api, which means moving the kit's painter and
   the sections wrappers there too?
