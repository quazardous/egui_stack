# egui_stack changelog

## egui_stack 0.29.0 - 2026-11-19

### Breaking changes

- Behaviour change: the strip of a zone the solver folded short of room (`Shown::AutoFolded`) is no longer a dead control. A click opens the zone over its neighbour, as `Layout::set_over` does, and closing it (its strip, Escape or a press outside) puts it back in its automatic state, so it unfolds by itself once the room comes back (`ZoneState::over_from_auto`). A new hook, `ZoneViewer::on_strip(zone, shown, response) -> StripAction`, the strips' counterpart of `TabViewer::on_tab_button`, lets the application choose what a strip click does (`Default`, `Unfold`, `Over`, `Fold` or `Nothing`). Every strip click is reported as `LayoutEvent::StripClicked`, an opening from the strip as `LayoutEvent::OpenedOver`, a closing back to the automatic state as `LayoutEvent::ClosedOver`, and with the `bus` feature as `Change::StripClicked`, `OpenedOver` and `ClosedOver` on `stack.layout.changed` (#149). Migration: an application that wants auto-folded strips to stay inert returns `StripAction::Nothing` from `ZoneViewer::on_strip` for `Shown::AutoFolded`; one matching every `LayoutEvent` or `Change` adds the new variants.
- Behaviour change: `DockSpace::take_unfolds` now also reports a folded zone holding a single dock
  when a tab is dropped on its strip (`Constraints::strip_drops`), as it already did for a folded
  split zone; before, such a drop was not reported. egui_stack still unfolds nothing itself: an
  application that unfolds every zone `take_unfolds` returns now unfolds single-dock zones on a
  strip drop too, and one that wants them to stay folded leaves them out. With the `bus` feature,
  the drop names the zone as well as the dock: `DropAsk` has a new field, `zone` (the folded or
  collapsed zone dropped on, else the zone holding the destination dock), and the `TabMoved`
  `LayoutChange` sets `zone` the same way (#147). Migration: apps that unfold everything
  `take_unfolds()` reports now also unfold single-dock zones; leave those zones out to keep them
  folded.

### Added

- A tab can leave a `DockSpace` and come back where it was, to hide and show it or to close and reopen it in place: `DockSpace::take_with_place` removes a tab and returns it with its `TabPlace` (its dock, the tabs next to it and its index in its group; opaque, and serialisable with the `serde` feature), and `DockSpace::restore_at` puts it back, beside its neighbours, else in its dock's focused group at its old index, else where a float's tab goes when the float closes (`ClosePolicy::ReturnToOrigin`). No hidden tab stays in the tree (#141). Restoring now shares one neighbour search with closing a float: it looks for the tab that was before the restored one in every group of the dock first, and only then for the tab that was after it (it used to try both in each group in turn). So a tab coming back from a closed float (`ClosePolicy::ReturnToOrigin`) now lands after its left neighbour wherever that tab has gone, where it could land before its right neighbour in another group.

### Fixed

- The Move group menu no longer shows a float's dock the space named itself (`float-N`, made when tabs are moved out to a new float) by that name: where `TabViewer::dock_title` gives no title, the item reads the float's window title, the active tab's title of its focused (else first) group, and follows it when the active tab changes. A dock the application named keeps its name, and `dock_title` still takes precedence (#150).
- Documented: dock names `float-<digits>` and `window-<digits>` are reserved for the docks a `DockSpace` makes itself for floats (`float-N` for tabs moved out to a new float, `window-N` from `DockSpace::windows_to_floats`). The Move group menu recognises such a dock by its name and shows it by its float's window title where `TabViewer::dock_title` gives none (#150), so a dock the application names so is shown the same way. See `DockSpace::add_dock`.

## egui_stack 0.28.9 - 2026-11-19

### Added

- A float's move handle is no longer a keyboard stop that does nothing: Tab passes it by. The new `TabViewer::on_window_handle` gives the application the handle's response, as `on_tab_button` does for tabs, so it can make it focusable and wire keys itself. Setting a shown float's `Place` in the layout from code now moves its window there, over the main window and natively, with no `FloatEvent::Moved`; it was only read when the window opened.
- `DockState::set_collapsed(path, collapsed)` and `Tree::set_leaf_collapsed(node, collapsed)` collapse or expand a tab group from code as its collapse button does, keeping the tree's collapsed state up to date: a float whose groups are all collapsed shrinks to its tab bars and grows back once one is expanded. `Node::set_collapsed` sets the node's flag only, and left such a float shrunk; its docs point to them. `DockState::focus_tab` does not expand a collapsed group (#142).
- Dock tabs follow the accessible tab pattern: the active tab's body is a `TabPanel` under its leaf, named by its tab, which `controls` it, and each tab list has a name, the dock's title (`TabViewer::dock_title`) or the new `Labels::tab_list` ("Tabs" by default) to translate. Without its tab bar, the panel takes the tab's title as its name.

### Fixed

- A float over the main window without a title bar of its own (the default since 0.28.7) has a "Close window" button beside its move handle: it closes the float as the title bar's close button does, asking `TabViewer::close_policy` what becomes of its tabs, even when they cannot be closed one by one, which left such a float with no way to close it (#143).
- A `LayoutArea` zone's content now lies inside its pane in the accessibility tree: the content's `Ui` is the `Pane` named after the zone's title, where it was an empty `Pane` beside its controls. A zone lying over its neighbour does the same, and a flyout's content is a `Pane` named by `ZoneViewer::flyout_title`.

## egui_stack 0.28.8 - 2026-11-18

### Fixed

- A float over the main window whose tab groups are all collapsed shrinks to its tab bars, and hides its resize grip; expanding a group gives it back the height it had, which its place keeps meanwhile, so a layout saved collapsed expands to it too. It kept its full height before, its body empty. A native float keeps its window's size, the system's to set.

## egui_stack 0.28.7 - 2026-11-18

### Added

- A float over the main window no longer wears a title bar of its own above its dock: its top tab bar is its top, its move handle moves it and the close all button of its tab bars closes it, as the legacy windows looked before the demo's first page moved to `FloatsArea` in 0.28.5. `FloatsArea::title_bar(true)` brings the title bar back (the dock's title and a close button, which moves the window too); a float whose move handle is turned off keeps it, to be moved by. Native floats keep the system's decoration.

### Fixed

- Every button of a tab bar paints the same way, through one helper: the line under the bar now runs under the collapse button and a float's move handle too (it stopped short of them), no button's highlight covers it any more (the group grip's, the collapse button's and the move handle's did), and the move handle has the left border the add and close all buttons have. The group grip and the collapse button's symbol sit a point higher, centred above that line like the others'; `DockPainter::tab_bar_tail` is also called for the collapse button's slot, and `TabBarGripPaint::rect` leaves out the line.

## egui_stack 0.28.6 - 2026-11-18

### Added

- `TabViewer` has a new provided method, `dock_title`: the title the Move group menu shows, and gives as accessible name, for a dock of a `DockSpace`. By default `None`, which keeps showing the dock's name.

## egui_stack 0.28.5 - 2026-11-18

### Fixed

- The demo's first page, One dock, draws its floating windows with `FloatsArea`, as the Dock space page does: a tab dragged out of its dock opens a float with the window's move handle, over the window or natively, where it opened a legacy `egui::Window` before. The toolbar's New window is now New float, on both pages, and the Floats choice (in the window or native) is offered on both, natively only outside the browser. `DockState::add_window`'s documentation points to floats.
- A tab or group drag whose release the window never sees ends as a cancel, as on Escape: the layout stays as it was and the ghost goes, in one window or across the viewports of a `DockSpace`. On Wayland, a screenshot taken during a drag grabs the pointer and the button is let go while the compositor holds it; the drag stayed live, its ghost following the pointer. It is now cancelled when the window loses the focus, or when the button is pressed again while egui still believes it held; the drag stays stopped until the button is let go.
- A tab held by the pointer but not in the air (not yet past the drag activation distance, or dragged back onto its own slot) keeps its look at rest in its tab bar, joined to the bar with square bottom corners. It was painted with the dragged look, an outlined shape rounded on all four corners standing apart from the bar.

## egui_stack 0.28.4 - 2026-11-18

### Added

- The application shapes the docks of its floats: `FloatsArea::painter` paints them with its own `DockPainter` (tab bars and window outlines), `FloatsArea::configure` sets each float's `DockArea` options (no splits, no close or add buttons), and `FloatsArea::decorate` draws over each float once its dock is drawn, knowing where its tab bars are (`FloatShown`, also `DockArea::tab_bars`). `DockSpace::carry_floats_from` keeps the floats' windows and the drag in progress across a space rebuilt from the application's own layout.

## egui_stack 0.28.3 - 2026-11-18

### Added

- `TabViewer::ghost_size` and `TabViewer::ghost_body` let an application size the ghost of a dragged tab or group and paint its body, a live preview of the tab's content under the ghost's tab bar for instance (a group shows its active tab). By default neither changes anything; a body the application paints replaces the painter's plain hint, inside the same frame and shadow.
- A dragged tab leaves its bar and shows the same ghost as a dragged group, with just that tab: shadow, translucency, tab bar, body hint and size limits, painted by the new `DockPainter::dragged_tab`, whose default paints it as `dragged_group` paints a one-tab group (`DraggedGroupPaint` has a new field, `body_painted`). In a space, the window under the pointer paints it, and the window the drag began in paints nothing of it meanwhile. A tab dragged by the body of a leaf whose tab bar is hidden gets the ghost too.

### Fixed

- The buttons at the right end of a tab bar sit side by side again: with the add button after the last tab (the default since 0.28.2), its empty slot no longer leaves a gap between the group grip and the close-all button. The slot is now left of the grip, where the add button goes once the tabs fill the bar or with `TabAddAlign::Right`.

## egui_stack 0.28.2 - 2026-11-18

### Added

- Floating windows get gestures split by where a drag starts: a tab drags the tab, the group grip or the group modifier drags the group, and the title bar or a new move handle moves the window. The move handle (`GroupHandles::MOVE_WINDOW`, on by default in `GroupHandles::ALL`, named by `Labels::move_window`) sits at the right end of a float's top tab bar, shows the move cursor, and moves the window: through the system on a native float (`ViewportCommand::StartDrag`), as the `egui::Window` on a float over the main window. `FloatsArea::float_move` chooses what moving a window does: `FloatMove::Move` (the default) only moves it and never docks; `FloatMove::Dock` docks the whole float into the dock it is released over, highlighted while it moves, after the viewer's veto (or `stack.dock.drop`), and falls back to a plain move where window positions are hidden (Wayland). `FloatsArea::group_handles` and `group_drag_modifiers` set the floats' handles and modifiers. A float over the main window is moved by its title bar only, never by the empty part of a tab bar. The add tab button now sits right after the last tab by default (`TabAddAlign::Left`, the new default of `ButtonsStyle::add_tab_align`); `TabAddAlign::Right` keeps it at the end of the bar.

### Fixed

- A whole tab group being dragged (by its grip, the empty part of its tab bar, Shift and a tab, or a `DockArea::drag_group_by` handle) shows its ghost under the pointer, as a dragged tab does: its tab bar with its tabs, the active one marked, and a hint of its body, kept to a sensible size with a "+N" for the tabs that do not fit, in whichever window of a `DockSpace` the pointer is over. It is painted by a new provided method, `DockPainter::dragged_group`, from a `DraggedGroupPaint` (with `DraggedGroupTab` and `DraggedGroupMore`), so an application can restyle it.

## egui_stack 0.28.1 - 2026-11-18

### Added

- A tab or a whole group is dragged across native windows in one drag (`DockSpace::dragging`, `SpaceDrag`, `DragPayload`): from the main window to a native float, back, or from float to float, with the same drop zones, veto (asked when the destination changes and at release) and Escape, across screens of different scales; released over nothing it opens a new float, a native float's tab torn off included; closing the window under the pointer cancels the drag; where window positions are hidden (Wayland), the windows that could take it outline their docks (`DockPainter::window_outline`, `WindowOutlinePaint`) and the first to see the pointer after the release takes it. A tab that moves into a float's dock records its origin, and forgets it back in a dock. `TabDestination` and `TabInsert` are `Clone`, `Debug` and `PartialEq`.

## egui_stack 0.28.0 - 2026-11-18

Native floating docks ([#105](https://github.com/quazardous/egui_stack/issues/105)): tabs dragged
out of the window open in floats of the layout, over the main window or in native windows the
application hosts, and go back where they came from when closed; and tab groups moved by a grip,
a modifier drag or a menu. Dragging a tab from one native window to another is to come.

### Breaking changes

- The layout holds floating roots, ahead of native floating docks (#105): `layout::Float` (a key, a dock, a host `FloatHost::InWindow` or `Native`, a `Place`, a z, minimized), `Layout::floats` and the places of closed floats (`Layout::closed`, `open_float` reopens where it was), `Screen` and the pure `Place::repair` (re-projected to the screen's scale, shrunk to fit, title bar kept on a screen, `ScreenGone::MoveOnto` or `DockBack` when its screen is gone), and `DockSpace::record_origin` keeping a tab's dock and neighbours (`TabOrigin`). The saved layout is version 2: a `Docked` saved by 0.27 or earlier turns its floating windows into docks `window-N` shown by native floats, and a version 2 layout does not load in 0.27. Migration: a `Layout` built field by field adds `floats: Vec::new()` and `closed: BTreeMap::new()` (or use `Layout::new`).
- A whole tab group can be moved by handles chosen with `DockArea::group_handles`, a set of `GroupHandles` flags, all on by default: `EMPTY_SPACE` (the empty part of the tab bar, as before), `GRIP` (a grip always present at the end of each tab bar, so a bar full of tabs can still be moved; named by `Labels::move_group`, painted by the new `DockPainter::tab_bar_grip` with `ButtonsStyle::group_grip_color`, `group_grip_active_color` and `group_grip_bg_fill`), `MODIFIER_DRAG` (a tab dragged with `DockArea::group_drag_modifiers` held, Shift by default, carries its group) and `MENU` ("Move group" in the tab's context menu and the grip's menu: a new window, or another dock of a `DockSpace`). Each follows the drop zones, the viewer's veto and Escape as a tab drag does. Tab bars are 16 points narrower for their tabs, the grip's room. Migration: a `TabContextMenuTranslations` built field by field adds `move_group_menu` and `move_group_new_window`, and a `ButtonsStyle` built field by field adds the three grip colours (or ends with `..Default::default()`); `.group_handles(GroupHandles::ALL.with(GroupHandles::GRIP, false))` keeps the former tab bars.
- Native floating docks (#105, step 3): `FloatsArea` (or `Docked::floats`) draws the layout's floats each frame, over the main window as `egui::Window`s or in native windows through a `ViewportHost` the application implements (defaults on egui's immediate viewports: `EguiViewports`). Once floats are drawn, a tab or a group dragged out of every dock, ejected, or moved to a new window becomes a float in a dock of its own with its origin recorded, not a legacy `Surface::Window` (legacy windows found in the space become floats); `DockSpace::float_out` and `float_out_group` do it from code, and `DockSpace::set_float_host` picks the host. Closing a float asks `TabViewer::close_policy` for a `ClosePolicy` (`ReturnToOrigin` by default, `Into(dock)`, `CloseTabs`, `Keep`). `DockSpace::take_float_events` reports `FloatEvent::{Detached, Closed, Moved, Repaired}`; native floats follow their window and are brought back onto the host's screens when they change. `Layout::repair_floats` now leaves `InWindow` floats alone. The optional `bus` feature (bus-contract 0.3.2) declares egui_stack's points on an application's bus with `DockSpace::with_bus`: `stack.dock.drop` (fail-closed, replaces `dock_drop_allowed`, asked when the destination changes and at release), `stack.layout.changed`, `stack.window.detached`, `stack.window.close` (fails open to the default), `stack.window.closed` and `stack.window.repaired`. `DockSpace::windows_dock` and `set_windows_dock` are deprecated. Migration: a `TabViewer` matching every method by name has nothing to do; an application drawing a `Docked` calls its floats after its docks to see the tabs dragged out.

## egui_stack 0.27.1 - 2026-11-18

Titles on folded strips ([#104](https://github.com/quazardous/egui_stack/issues/104)), opt-in: a
layout that leaves `LayoutStyle::strip_title` unset looks and behaves as before.

### Added

- Folded strips can show their zone's title: set `LayoutStyle::strip_title` to a `StripTitle` (off by default). Left and right strips read the same way, from the bottom up unless `StripTitle::vertical` says `VerticalReading::TopToBottom`; a long title is cut with an ellipsis and shown whole in a tooltip; the strip grows to fit the font, an optional icon (`ZoneViewer::strip_icon`) and the padding. `ZoneViewer::strip_title(ui, zone, rect, orientation)` draws the title instead, in the room past the grip, while the layout keeps the strip's clicks, drag and drops.

## egui_stack 0.27.0 - 2026-11-18

Collapsible docks ([#103](https://github.com/quazardous/egui_stack/issues/103)). The one break is
`OverlayFeel`'s new public field, `restore_band`: a struct literal naming every field ends with
`..Default::default()`. Nothing changes for a layout that does not opt in with `Fold::Collapse`.

### Breaking changes

- Collapsible docks: a zone with the new `Fold::Collapse` collapses to nothing when folded (`Shown::Collapsed`), keeping its place in the tree, its constraints and its last size, and `DockSpace::collapse_empty` collapses those whose docks are empty, so the zone that grows takes all the room. While a tab is dragged, each collapsed zone shows a thin restore band at its former boundary (on its parent's outer edge when nothing is left between them; several on one boundary stack in tree order), painted by the new `DockPainter::restore_band`; hovered, it previews where the zone comes back (`LaidZone::restore`), and a tab released there, unless vetoed, joins the zone's dock and the zone is named in `DockSpace::take_unfolds` to unfold at its remembered size within its current bounds. A cancelled drag or a release elsewhere leaves the layout unchanged. The band's breadth is the new `OverlayFeel::restore_band`, 8 points by default. Migration: an `OverlayFeel { .. }` literal needs `restore_band` or `..OverlayFeel::default()`; a `match` on `Shown` already has a catch-all arm (the enum is `#[non_exhaustive]`).

## egui_stack 0.26.0 - 2026-11-17

`OverlayFeel` gains public fields, which breaks code that builds it with a struct literal naming
every field (end the literal with `..Default::default()`). Behaviour changes to know before
upgrading: a `DockArea` separator's accessible name is now "Resize between Left and Right" (after
its neighbours) instead of "Resize", so a test or an assistive script that looked it up by
"Resize" must use the new name; a tab starts dragging 6 points from the press in every direction
(`OverlayFeel::drag_activation_distance`), sooner than before; and with `OverlayType::Zones` a
leaf's drop zones leave its tab bar out.

### Breaking changes

- With `OverlayType::Zones`, the drop zones of a leaf leave its tab bar out: `OverlayFeel` has two new fields, `zone_tab_bar_stack_band`, a band right under the tab bar where a dropped tab joins the leaf's tabs as over the tab bar (`8.0` points by default), and `zone_edges_below_tab_bar`, which lays the edge bands on the leaf's body below the tab bar and that band (`true` by default) instead of on the whole leaf, where in a compact leaf the top band was mostly hidden under the tab bar, leaving a strip under it that split the leaf. `DropZone::in_leaf` picks a zone the way the dock does. `zone_edges_below_tab_bar: false` with `zone_tab_bar_stack_band: 0.0` gives the former geometry back. Breaking for code that builds `OverlayFeel` with a struct literal naming every field: add the new field, or end the literal with `..Default::default()`.
- `OverlayFeel` has a new field, `drag_activation_distance`: how far, along `x` and `y`, the pointer moves from where it pressed a tab (or the empty part of a tab bar) before the tab (or the tab group) starts being dragged, measured from the press. It is `6.0` along both axes by default, egui's click distance, so a tab drags as soon as the press is no longer a click, in every direction; it used to start only 30 points to the side or 6 points up or down past that point, about 38 and 14 points from the press, with a range in between where the tab neither clicked nor dragged. `vec2(37.0, 13.0)` gives the former distances back. The dragged tab's title moves with the pointer from where it was pressed. Breaking for code that builds `OverlayFeel` with a struct literal naming every field: add the new field, or end the literal with `..Default::default()`.

### Added

- A zone the solver folds by itself for want of room (`Constraints::auto_fold`) is reported once in `Laid::events` by `LayoutArea`: `LayoutEvent::AutoFolded` when it folds, `LayoutEvent::AutoUnfolded` when the room comes back. Its `ZoneState::folded` is still not touched, so an application tells a zone the user folded (`Folded`, `Unfolded`) from one folded for lack of room. `Layout::set_folded` still reports no event: the application made that change itself. An application matching `LayoutEvent` already has a catch-all arm (the enum is `#[non_exhaustive]`).
- An editor whose tab bar is hidden can be moved again by a handle of the app's choosing: `DockArea::drag_group_by(ui, &response)`, called from `TabViewer::ui` with the response of a widget that senses drags (an editor's own header, a grip), makes that widget drag the whole tab group, as the empty part of a tab bar does, through the same drop zones and past `OverlayFeel::drag_activation_distance`. `DockArea::hidden_tab_bar_drags_group(true)` makes the strip at the top of a hidden tab bar's leaf (`TabBodyStyle::hidden_tab_bar_drag_height` high) drag the group too, instead of its active tab alone. Both are opt-in: nothing changes for an app that uses neither.
- A `DockArea` separator is named after what it resizes, "Resize between Left and Right" (the titles of the active tabs on each side, of the first leaf for a side holding a split), from the new label `Labels::separator_between`, so a screen reader tells two separators apart; `Labels::separator` ("Resize") still names a grid's lines and a separator next to an empty leaf. A `LayoutArea` sash now exposes its step, `LayoutStyle::key_step`, next to its value, minimum and maximum in points, as a `DockArea` separator does. Both stay Tab stops moved by the arrow keys, and ranges set or stepped through accesskit (SetValue, Increment, Decrement). An app or test that looked a dock separator up by the name "Resize" finds it by its new name.
- A folded split zone's strip can take dragged tabs: with `Constraints::strip_drops` on a split zone, a tab released on its strip joins one of the docks the split holds that accept drops, the one holding the focused leaf (the last active), else the first, depth first, and `DockSpace::take_unfolds`, called after the layout is drawn, names the zone for the application to unfold with `Layout::set_folded`. `LaidZone` has a new field, `docks`: the docks a split's leaves hold. Off by default: a split zone without `strip_drops` behaves as before.

### Fixed

- A dragged tab lands where the pointer is released, not where it was a frame earlier: released on a leaf's tab bar it joins the leaf's tabs, released in an edge band it splits the leaf, also when the pointer moved there in the release frame or while the hovered leaf is still preferred (`OverlayFeel::max_preference_time`), which now keeps the leaf only, not whether its tab bar or its body was hovered.
- A folded split zone's strip that takes no drops (without `Constraints::strip_drops`, or when none of its docks accepts them) is a no-drop zone for `DockSpace::bind`, as a folded leaf's strip is: a tab released there stays where it was, instead of opening in a floating window.

## egui_stack 0.25.12 - 2026-11-16

### Added

- A whole tab group can be dragged by the empty part of its tab bar, as an editor group is dragged by its tab bar: all its tabs move together, in order, its active tab kept, with the same drop zones as a tab (the middle of a leaf joins its tabs, an edge band splits it, outside the dock a new window), and the viewer's `dock_drop_allowed` asked for each of its tabs. `DockArea::dragged_node` reports the drag (`DraggedNode`), and `DockState::move_node` moves a leaf whole as `move_tab` moves a tab.

## egui_stack 0.25.11 - 2026-11-12

### Added

- `LayoutPainter::over_edge` paints the free edge of a zone lying over its neighbour, the edge that resizes it, with its state (hovered, pressed, dragged) in a `SashPaint`, above the zone's content. By default it paints nothing, as before; a host that styles its sashes' hover and drag can paint the same line there, so a docked zone's sash and a zone lying over look alike.

## egui_stack 0.25.10 - 2026-11-12

### Fixed

- A docked zone's content follows its sash in the same frame while the sash
  is dragged: `LayoutArea` applies the sashes' moves before it draws the
  zones, and paints the sashes from the same layout, so the content's edge no
  longer trails the sash line by one frame.

## egui_stack 0.25.9 - 2026-11-12

### Fixed

- A click anywhere on a folded zone's strip now unfolds it (or folds back a zone lying over), as its grip does, wherever the strip's own widgets do not take the click: an empty strip is a button as a whole. The band at the strip's edge that drags the zone out takes a press without motion as a click too, and on a strip narrower than twice `LayoutStyle::sash_grab` it lies just outside the strip instead of covering it, so the resize cursor shows only there.

## egui_stack 0.25.8 - 2026-11-11

### Added

- `LayoutArea::sublayer_of(parent)` draws the zones lying over their neighbour and the flyout as sublayers of `parent`, a foreground layer the host holds: egui keeps them directly above it whatever was clicked last, so a host's own overlays shown above that layer stay above them. By default they are plain foreground areas, raised when clicked, as before.

## egui_stack 0.25.7 - 2026-11-11

### Fixed

- In a stack sized by its content (`Sections::fill(false)`), a drag moves a band again when the open sections above are at their content's height: down, the section above grows past its content, into the room under the last band; up, it shrinks to its least height. The last open section keeps the height a drag gives it, and the new `plan::fit_content` and `plan::pin_plan_content` lay such a stack out.

## egui_stack 0.25.6 - 2026-11-10

### Added

- `LayoutArea::flyout_dismiss` and `LayoutArea::over_dismiss` take a `Dismiss` saying what closes the flyout and what folds back a zone lying over its neighbour: a press outside, Escape (`Dismiss::ANY`, the default as before), one of them, or neither (`Dismiss::NEVER`, lying over then lasts until its grip folds it). `LayoutArea::inside` lets the host count a press outside as inside, from its point and the egui layer on top there, so that a combo box's list, a context menu or the host's own overlays keep the flyout open. A host that closes its own overlays first on Escape turns `escape` off or consumes the key before the area is drawn.
- A zone lying over its neighbour resizes from its free edge, as a flyout does: a splitter named as its sash ("Resize left"), within its least and largest sizes, written to its `ZoneState::size` and reported as `LayoutEvent::Resized`.

## egui_stack 0.25.5 - 2026-11-10

### Added

- `Docked<Tab>` keeps a `Layout` and its `DockSpace` as one value (saved together with the `serde` feature), and `Docked::repair` makes them agree: zones showing a dock the space lost go, docks no zone shows get a hidden zone. `Layout::from_space` builds a layout for a `DockSpace` placed by egui panels, from a tree shape the application gives.
- A folded zone's strip edge facing its neighbour drags the zone out: past half its least size, it unfolds on release at the size dragged to, with a guide line meanwhile; `LayoutStyle::strip_drag` turns it off.
- A layout zone whose least size is its largest has no sash: nothing to resize, and no keyboard stop.
- A `LayoutArea`'s zones are named after their title, `ZoneViewer::title` (by default the zone's name), and its sashes, grips and flyout edge after `Labels::resize_zone`, `unfold_zone`, `fold_zone` and `resize_flyout`, set with `LayoutArea::labels`, so an application translates them and its zones' names never clash with its own labels. `ZoneViewer::flyout_title` names a flyout.
- A dragged layout sash snaps to its zone's default size and to the line of another sash running the same way, within `LayoutStyle::snap` (8 points; 0 turns it off), with a guide line painted by `LayoutPainter::snap_guide` (`SnapGuidePaint`, `SnapKind`); Alt suspends it. A guide also shows where a zone will fold on release.
- `Constraints::strip_drops`: a folded zone's strip takes a dragged tab for the dock it holds, which joins that dock's focused leaf (else its first), through `DockSpace::bind`. Off by default: strips still refuse drops.

## egui_stack 0.25.4 - 2026-11-08

### Added

- `DockSpace::bind` puts a dock space on a layout: called each frame with the `Laid` a `LayoutArea` returned, it makes no-drop zones of the zones with `Drops::Refuse`, the zones holding no dock (`Content::App`, or a dock the space has not) and the folded zones' strips, so a tab released there goes back where it was; a `Content::Empty` zone that accepts drops stays outside every dock, where a tab becomes a floating window. Each dock is drawn in its zone by the `ZoneViewer`, with `DockArea::dock`.
- `layout::LayoutArea` draws a `Layout` in a `Ui`: each leaf zone's content through a `ZoneViewer`, as a pane named after the zone, and a sash between neighbouring zones that resizes the zone away from the one that grows, by drag, by the arrow keys (Shift for a larger step) or through assistive technology (a splitter whose value is the zone's size), within the zone's least and largest sizes and the growing zone's least size; Enter or a double click gives the zone back its default size. Zones with `fit_content` measure their content for the next frame. A `LayoutPainter` trait (with `DefaultLayoutPainter`) and a `LayoutStyle` let the host style the sashes; `Laid` lists the user's changes as `LayoutEvent`s, and each `LaidZone` its bounds, content and drop rule.
- A `LayoutArea` folds zones: a folded zone keeps a strip in the flow, painted by `LayoutPainter::strip` with a grip that unfolds it, and the application's icons from `ZoneViewer::strip`; a sash dragged under half a `Fold::ToStrip` zone's least size folds it on release. A zone may lie over its neighbour (`Layout::set_over`), next to its strip and without taking room, until Escape or a click outside folds it back. `LayoutArea::flyout` shows one `Flyout` of a folded zone over its neighbour (`ZoneViewer::flyout`), closing on Escape, a click outside or the grip (`LayoutEvent::FlyoutClosed`), with a free edge that resizes it; the breadths are kept per key in `Layout::flyouts`, saved with the layout as the zones' states are. `Layout::set_folded` and `set_over` change a zone's state from code.
- A new `layout` module models a window's whole space as a tree of zones (`Layout`, `Zone`, `Content`), with constraints set from code (least, largest and default sizes in points or shares, a zone that grows, priorities, folding) and the user's state, and solves it into rects (`Layout::solve`, `Laid`): the sides give way to the centre's least size, lowest priority and nearest first, never in proportion, and zones that opt in fold to a strip when even the least sizes do not fit. With `serde`, a layout saves its tree and state, versioned; `Layout::reconcile` re-asserts the code's constraints after loading and adds or drops the zones the code added or removed.

### Fixed

- A floating window gets back the height it had once it is unfolded (or restored from minimized): its least size no longer overrode the height kept when it was folded, so it stayed as short as its folded bands. A split window whose every leaf is folded collapses to its bands and, once a leaf is unfolded, gets back its height and its splits' shares; the height kept is saved with the layout.

## egui_stack 0.25.3 - 2026-11-07

### Added

- `DockSpace::no_drop_zone`, `DockSpace::no_drop_zones` and `DockSpace::no_drop_widget` declare, every frame, where no dragged tab may be dropped (an application's toolbar or status bar): a tab released there goes back where it was instead of becoming a floating window, and no drop preview shows there. `DockRules::accept_drops(false)` makes a dock of a space no drop target, its own tabs still free to leave it.
- `Section::max_height` gives a section a greatest height, in points, and `Section::max_share` one as a share of the stack's height: sized by the list or set by hand, the section never grows past it, the room it leaves going to the other sections, or staying empty under the last band when none can take it, and a drag on its boundary stops there. A greatest height under the section's least height is its least height.

### Fixed

- A tab dragged over a floating window drops into the window, not into the docked leaf or the dock under it, whatever the order the docks of a `DockSpace` are drawn in; with overlapping windows, the topmost one takes it.
- A floating window shrunk past what its content needs stops at its content's least size: each leaf keeps its tab bar with its buttons and a body, the separators come on top, and the tab bars and their buttons stay inside the frame instead of spilling past it.
- A floating window whose leaves are folded is tall enough for its folded bands: its height counts the window's frame and the separators between the bands, so the band's tabs and buttons are no longer clipped by the window's edge.

## egui_stack 0.25.2 - 2026-11-07

### Added

- `Sections::bar_style` gives the scroll bars of a stack's bodies and of the stack itself a style of their own (their colours: egui paints a bar from the visuals of the `Ui` its scroll area is shown in), the content keeping the `Ui`'s, and `Sections::bar_track_inset` keeps the thumb that many points from both ends of the track.
- `Sections::fill(false)` sizes a stack by its content: an open section sized by the list is never taller than what its body laid out, the long ones share what is left, the folded bands follow right under the last open section and the room left stays empty under the last band; sections set by hand, resizing, least heights and scrolling are as before. `sections::plan::fit_within` and `pin_plan_within` are its arithmetic.

## egui_stack 0.25.1 - 2026-11-07

### Added

- A `DockSpace` holds several named docks, each with its own tree, and the floating windows, sharing one drag: each dock is drawn by its own `DockArea::dock(&mut space, name)` call, anywhere in the frame (a side panel, the centre), and a tab dragged from any dock to any other gets the same drop zones (the middle of a leaf, its four edge bands, a slot in its tab bar), preview and veto as inside one dock. Each dock has its `DockRules`: the splits a drop may make in it, whether its tabs may become floating windows, and whether it stays as an empty drop target once its last tab leaves. `TabViewer::dock_drop_allowed` asks `drop_allowed` by default and names the docks the tab would leave and enter. A space serialises as one value with the `serde` feature, and `DockSpace::from_states` turns one `DockState` per area into a space. `DockState::retain_tabs` and `filter_map_tabs` now leave an emptied window's surface empty instead of shifting the surfaces after it, as `remove_surface` already did, so every surface keeps its index.

## egui_stack 0.25.0 - 2026-11-06

### Breaking changes

- The default look draws no line between two leaves at rest, where it drew three (the left body's border, the separator and the right body's border): `TabBodyStyle::stroke` is now `Stroke::NONE` in `TabBodyStyle::from_egui` (it already was in `Default`), and `SeparatorStyle::color_idle` is transparent in `SeparatorStyle::from_egui` and `Default`. The separator still shows on hover and drag in `color_hovered` and `color_dragged`, and its width and grab area are unchanged. Migration: for the former look, set them after building the style; from an `egui::Style` `s`, `style.tab.tab_body.stroke = s.visuals.widgets.noninteractive.bg_stroke;` and `style.separator.color_idle = s.visuals.widgets.noninteractive.bg_stroke.color;` (`Style::default()` had `color_idle: Color32::BLACK`).

### Added

- A leaf's body border is painted by the `DockPainter`, in a new provided method, `tab_body`, which gets a `TabBodyPaint`: the body's rect, its corner radius, the styles, and its state (`PartState::HOVERED` under the pointer, `PartState::FOCUSED` and `PartState::LEAF_FOCUSED` when its leaf is the focused one). A painter can then draw borders no style field gives: on some sides only, glowing, or on the focused leaf alone. `StylePainter` paints `TabBodyStyle::stroke` as before.

## egui_stack 0.24.2 - 2026-11-06

### Added

- The corners of a tab bar follow what is below it: a folded leaf, which shows only its tab bar, has its bar and tabs rounded at the bottom, with no line under them, and so does a dragged tab, while a bar over a body stays square at the bottom whatever the state of its tabs. The highlights of a tab's close button and of the bar's buttons (collapse, add, close all) are rounded where they meet a rounded corner of the tab or bar instead of sticking out square, and the close button's highlight no longer covers the tab's outline. `TabBarPaint`, `TabBarTailPaint` and `TabPaint` have a new field, `folded`, which tells a painter that the leaf is folded.

## egui_stack 0.24.1 - 2026-11-05

### Fixed

- Corners where two parts join are painted square: a leaf's body under its tab bar has square top corners, a tab bar and the tabs in it have square bottom corners, so the active tab merges into its body without a notch. Free corners keep the radius of the style; a dragged tab and a body whose tab bar is hidden keep all four.

## egui_stack 0.24.0 - 2026-11-05

### Breaking changes

- A grid container: `Node::Grid(GridNode)` lays out two or more children in rows and columns, row by row, with `GridLayout::Auto` (as many columns as the square root of the number of children, rounded up, the rule of egui_tiles' grid) or `GridLayout::Columns(n)`. Each column and each row has a share; a separator on each column line and each row line resizes the two beside it, by drag, by the arrow keys (Enter evens them) and by assistive technology, as a linear node's separators do. `Tree::arrange_as_grid` turns a row or a column into a grid and `Tree::arrange_as_linear` turns a grid back into a row or a column. A drop on the edge of a leaf inside a grid, or `Tree::split` on it, adds the new view to the grid next to the leaf, and `Tree::insert_child` adds a view to a grid at run time. A grid emptied down to one child gives way to it, an empty one is removed, and grids are saved and loaded with the `serde` feature. `Node` and `GridLayout` are now `#[non_exhaustive]`, so later container kinds will not break callers. Migration: a `match` on `Node` outside the crate needs a wildcard arm (`_ => ...`), which also covers `Node::Grid`; use `Node::children` to walk any container.
- The tree's splits are n-ary: `Node::Linear(LinearNode)` lays out two or more children in a row or a column, each with a share, and replaces `Node::Horizontal` and `Node::Vertical`. Splitting a leaf along its parent's direction, by a drop or by `Tree::split`, adds it to that row or column instead of nesting a split, and the separator between two neighbours resizes those two only (the model follows egui_tiles). `Tree::insert_child` adds a node to a row or column, `Tree::parent` and `Node::children` walk the tree, and node indices stay stable as the tree changes, except for the root, always at index 0. Trees saved by earlier versions load with the same layout, nested splits of one direction joined into one row or column. Migration: match `Node::Linear(linear)` instead of `Node::Horizontal(split)` or `Node::Vertical(split)`, with `linear.dir` for the direction and `linear.boundary(0)` for a two-child split's former `fraction`; `SplitNode`, `Node::split`, and the heap arithmetic `NodeIndex::{left, right, parent, level, is_left, is_right}` and `NodePath::{left_node, right_node}` are gone: use the indices `Tree::split` returns, `Tree::parent` and `Node::children`.

## egui_stack 0.23.0 - 2026-11-05

### Breaking changes

- A new drop overlay, `OverlayType::Zones`, is the default: while a tab is dragged over a leaf, the region it would take (the half along an edge when the pointer is in that edge's band, the whole leaf elsewhere) is drawn on the leaf itself with a border and a light tint, with no buttons; dropped outside the dock, a tab allowed in windows opens in a new window. Its look is `OverlayStyle::zone_fill`, `zone_stroke` and `zone_corner_radius`, the width of the edge bands `OverlayFeel::zone_edge_coverage`, and `DropZone::at` is its hit-test, public for applications drawing their own drop targets. `OverlayType` is now `#[non_exhaustive]`. Migration: a `match` on `OverlayType` needs a `_` arm; a `Style`, `OverlayStyle` or `OverlayFeel` built field by field takes the new fields (or `..Default::default()`); to keep the former overlay, set `style.overlay.overlay_type = OverlayType::Widgets`.

## egui_stack 0.22.3 - 2026-11-04

### Added

- A toolbar row, `toolbar::ToolRow`: a row or a column of tools the caller gives (`Tool::button`, `toggle`, `radio`, `menu`, `separator`, `spacer` and `custom`), showing icons, labels or both (`ToolDisplay`), with tooltips and key bindings (`Tool::shortcut`), the tools that do not fit going in their order into a More button that opens them as a menu. It says which tool was clicked and holds no state of its own. It is a named `Toolbar` for screen readers, with toggles announced as toggled and busy tools as busy, and one Tab stop with the arrows, Home and End along it (`TabFocus::Roving`, its default). `toolbar::laid_out`, `focused` and `focus` read the row as drawn and move the focus in it. It is painted by the `DockPainter`, through three new methods with defaults, `tool_item`, `tool_more` and `tool_separator`; `PartState::DISABLED` is new, and `Labels` has two new names, `more_tools` and `more_tools_in`.

## egui_stack 0.22.2 - 2026-11-04

### Added

- A new `sections` module: `Sections`, a stack of collapsible sections filling a panel, each a title band over its own scrolling body (anything, windowed rows or the application's own view), folded by a click or the keyboard, resized by dragging the line above a band (sections give way in turn, the nearest first), with an optional pull that moves the sections below as a block, titles that drag or stay in place, accessible names and roles, and a `Layout` remembered in egui's memory by the stack's name (persisted with the `serde` feature); `sections::plan` holds its egui-free arithmetic. `DockPainter` gains `section_band` and `section_boundary`, with defaults painted from egui's `Style`, and `Labels` gains `resize_section`.

## egui_stack 0.22.1 - 2026-11-04

### Added

- A `DockPainter` trait paints the dock's parts, set with `DockArea::painter`: `tab_width` measures a tab (padding and
  close button included, so a tab can be narrower than its title plus 16 points), `tab_bar` and `tab_bar_tail` paint
  a tab bar under and after its tabs, `tab` a tab with its close button, `separator` a separator, and `drop_target`,
  `drop_button` and `drop_preview` the drop overlay, a refused drop included. Each part gets its rect, its state as
  `PartState` flags (selected, hovered, pressed, focused, dragged, leaf focused) and the style. Every method defaults
  to the painting from `Style`, the `StylePainter`, so the look is unchanged and a painter overrides only the parts it
  paints. The tab bar buttons, the hidden tab bar's handle, the scroll bar and the tab bodies are still painted from
  `Style`.

## egui_stack 0.22.0 - 2026-11-03

Forked from egui_dock 0.21.1 (see NOTICE). Everything below this release is egui_dock's history.

### Breaking changes

- The crate is renamed `egui_stack`.
- egui 0.37 (from egui_dock pull request #354 by crumblingstatue: version bump and deprecations fixed).
- `TabViewer::context_menu` gets the tab's `TabPath` instead of its leaf's `NodePath`, so a menu item can name the tab.
  Migration: change the last parameter's type to `TabPath`; `path.node_path()` gives the former `NodePath`.
  ([#9](https://github.com/quazardous/egui_stack/issues/9))
- `TabStyle` has a new public field, `padding`: the space on each side of a tab's title, `8.0` by default as before,
  which can be lowered for narrower tabs. Migration: a `TabStyle { .. }` literal needs `padding` or
  `..TabStyle::default()`. ([#12](https://github.com/quazardous/egui_stack/issues/12))

### Added

- Tabs are exposed to AccessKit with the role `Tab`, their title as name and `selected` when active, inside a
  `TabList` per tab bar. ([#2](https://github.com/quazardous/egui_stack/issues/2))
- The close, add, close-all, collapse, minimize, restore and show-tab-bar buttons have accessible names, from a new
  `Labels` struct with English defaults, set with `DockArea::labels`. The collapse button reports whether its leaf is
  expanded. ([#3](https://github.com/quazardous/egui_stack/issues/3))
- Separators are exposed to AccessKit as `Splitter`s named by `Labels::separator`, with the split fraction as value and
  the increment, decrement and set-value actions. ([#3](https://github.com/quazardous/egui_stack/issues/3))
- `TabViewer::drop_allowed(tab, destination)` returns a `DropVerdict` (`Allow` by default, or `Refuse` with an optional
  reason): a refused destination is drawn as refused on the drop overlay, with its reason next to the pointer, and
  releasing the tab there leaves the layout unchanged. ([#4](https://github.com/quazardous/egui_stack/issues/4))
- `DockArea::dragged_tab(ctx, id)` returns the tab being dragged (its `TabPath`) and the pointer's position, as a
  `DraggedTab`, so callers need not rebuild the tab buttons' private ids; `DockArea::default_id()` is the id of a
  `DockArea` whose id was not set. ([#5](https://github.com/quazardous/egui_stack/issues/5))
- A focused separator moves with the arrow keys along its axis, 16 points per press or 64 with Shift, and Enter resets
  it to the middle. Before, it moved only with Ctrl or Shift held, by 16 points either way, and with any arrow.
  ([#7](https://github.com/quazardous/egui_stack/issues/7))
- `DockArea::tab_focus(TabFocus::Roving)`: one Tab stop per tab bar, its active tab; Left and Right select and focus
  the previous and next tab, Home and End the first and last, and close buttons leave the Tab order. `TabFocus::Each`,
  the default, keeps every tab a Tab stop. ([#10](https://github.com/quazardous/egui_stack/issues/10))
- `DockArea::show_builtin_tab_menu(false)` hides the built-in items of the tab context menus (Eject, Close, Hide or Show
  tab bar), so the viewer's own items stand alone. ([#8](https://github.com/quazardous/egui_stack/issues/8))
- `DockState::close_tab(path, tab_viewer)`, `DockState::focus_tab(path)` and `DockState::eject_tab(path, position)` do
  what the dock's close buttons, tab clicks and Eject menu item do, so an application can offer them, with the existing
  `DockState::move_tab`, from its own menus and commands. ([#9](https://github.com/quazardous/egui_stack/issues/9))

### Fixed

- The tab context menu's Close asks `TabViewer::on_close` once, not twice. ([#1](https://github.com/quazardous/egui_stack/issues/1))
- `WindowState::rect` and `WindowState::dragged` report the window's rect and drag as last shown; they always returned
  `Rect::NOTHING` and `false`. `DockState::get_window_state` takes `&self` instead of `&mut self`, and both window-state
  getters return `None` for a surface that does not exist instead of panicking.
  ([#11](https://github.com/quazardous/egui_stack/issues/11))

# egui_dock changelog (before the fork)

## egui_dock 0.21.1 - 2026/08/06

### Fixed

- `DockState` serialised by previous versions of egui_dock failing to deserialise in 0.21.0.
  ([#337](https://github.com/anhosh/egui_dock/pull/337))

## egui_dock 0.21.0 - 2026/08/05

### Added

- Hiding/showing tab bars for a given leaf. ([#322](https://github.com/anhosh/egui_dock/pull/322))

### Breaking changes

- Upgraded to egui 0.36.
- Updated MSRV from 1.92 to 1.95.
- Implementing `TabViewer::id()` is now mandatory. ([#324](https://github.com/anhosh/egui_dock/pull/324))

## egui_dock 0.20.1 - 2026/06/28

### Fixed

- "Widget changed layer_id" panic when undocking tabs. ([#318](https://github.com/anhosh/egui_dock/pull/318))
- Translations are no longer serialised. ([#319](https://github.com/anhosh/egui_dock/pull/319))

## egui_dock 0.20.0 - 2026/06/27

### Breaking changes

- Upgraded to egui 0.35. ([#326](https://github.com/anhosh/egui_dock/pull/326))

## egui_dock 0.19.1 - 2026/03/31

### Fixed

- Corrected outdated documentation. ([#326](https://github.com/anhosh/egui_dock/pull/326))

## egui_dock 0.19.0 - 2026/03/29

### Breaking changes

- Upgraded to egui 0.34. ([#314](https://github.com/anhosh/egui_dock/pull/314))
- Replaced all `SurfaceIndex`, `NodeIndex`, and `TabIndex` groupings with `NodePath` and `TabPath` in the argument lists
  of the following functions ([#312](https://github.com/anhosh/egui_dock/pull/312)):
    - `DockState::set_active_tab`,
    - `DockState::set_focused_node_and_surface`,
    - `DockState::move_tab`,
    - `DockState::detach_tab`,
    - `DockState::remove_tab`,
    - `DockState::remove_leaf`,
    - `DockState::split`,
    - `DockState::focused_leaf`,
    - `TabViewer::context_menu`,
    - `TabViewer::on_add`,
    - `TabViewer::add_popup`.
- Changed the return type of some methods ([#312](https://github.com/anhosh/egui_dock/pull/312)):
    - `DockState::iter_all_nodes` now returns `impl Iterator<Item = (NodePath, &Node<Tab>)>`,
    - `DockState::iter_all_nodes_mut` now returns `impl Iterator<Item = (NodePath, &mut Node<Tab>)>`,
    - `DockState::iter_all_tabs` now returns `impl Iterator<Item = (TabPath, &Tab)>`,
    - `DockState::iter_all_tabs_mut` now returns `impl Iterator<Item = (TabPath, &mut Tab)>`,
    - `DockState::iter_leaves` now returns `impl Iterator<Item = (NodePath, &LeafNode<Tab>)>`,
    - `DockState::iter_leaves_mut` now returns `impl Iterator<Item = (NodePath, &mut LeafNode<Tab>)>`,
    - `DockState::find_tab_from` now returns `Option<TabPath>`,
    - `DockState::find_tab` now returns `Option<TabPath>`,
    - `Surface::iter_all_tabs` now returns `impl Iterator<Item = ((NodeIndex, TabIndex), &Tab)>`,
    - `Surface::iter_all_tabs_mut` now returns `impl Iterator<Item = ((NodeIndex, TabIndex), &mut Tab)>`,
    - `Tree::set_active_tab` now returns `Result<()>` (`Err` if any of the indices are invalid),
    - `LeafNode::::set_active_tab` now returns `Result<()>` (`Err` if the tab index is invalid).
- `impl From<(SurfaceIndex, NodeIndex, TabInsert)> for TabDestination` was replaced with
  `impl From<(NodePath, TabInsert)> for TabDestination`. ([#312](https://github.com/anhosh/egui_dock/pull/312))

### Added

- `NodePath` and `TabPath` structs, useful for more consistent and terse indexing of nodes and tabs.
  ([#312](https://github.com/anhosh/egui_dock/pull/312))
- Indexing `LeafNode` with `TabIndex`. ([#311](https://github.com/anhosh/egui_dock/pull/311/))
- Indexing `DockState` using `NodePath`. ([#312](https://github.com/anhosh/egui_dock/pull/312))
- New methods ([#312](https://github.com/anhosh/egui_dock/pull/312)):
    - `DockState::node(_mut)`: returns a `Node` at a given `NodePath`,
    - `DockState::leaf(_mut)`: returns a `LeafNode` at a given `NodePath`,
    - `DockState::iter_surfaces(_mut)_indexed`: returns a `Surface` iterator paired with its `SurfaceIndex`,
    - `Surface::iter_nodes(_mut)_indexed`: returns a `Node<Tab>` iterator paired with its `NodeIndex`,
    - `Tree::leaf(_mut)`: returns a `Result<&LeafNode<Tab>>` at a given `NodeIndex`,
    - `Node::iter_tabs(_mut)_indexed`: returns a `Tab` iterator paried with its `TabIndex`.

### Fixed

- No more panics when a window is shrunk to zero available space. ([#309](https://github.com/anhosh/egui_dock/pull/309))
- No more panics while trying move a tab to the end of the list within the same leaf.
  ([#308](https://github.com/anhosh/egui_dock/pull/308))

### Deprecated

- `DockArea::show` - use `DockArea::show_inside` instead. ([#314](https://github.com/anhosh/egui_dock/pull/314))

## egui_dock 0.18.0 - 2025/10/31

### Breaking changes

- Upgraded to egui 0.33. ([#293](https://github.com/Adanos020/egui_dock/pull/293))

### Changed

- Node separators are always clamped between their bounds. ([#289](https://github.com/Adanos020/egui_dock/pull/289))

## egui_dock 0.17.0 - 2025/07/13

### Breaking changes

- From ([#272](https://github.com/Adanos020/egui_dock/pull/272)):
    - `Node`s underlying data has been split up into the `LeafNode` and `SplitNode` types, meaning that any match
      statements carried out on a node now needs to account for this.
- Upgraded to egui 0.32 ([#280](https://github.com/Adanos020/egui_dock/pull/280/))

### Changed

- From ([#272](https://github.com/Adanos020/egui_dock/pull/272)):
    - `Tree::set_active_tab` now takes `impl Into<NodeIndex>` and `impl Into<TabIndex>` to make use slightly easier.
    - `Surface` now implements `Index<NodeIndex>`/`IndexMut<NodeIndex>` which tries to access the surfaces node tree and
      the node at the index. This will always panic when used on an empty surface as they do not have a node tree nor
      nodes.

### Added

- From ([#272](https://github.com/Adanos020/egui_dock/pull/272)):
    - `DockState::iter_leaves` and `DockState::iter_leaves_mut` - can be used to more efficiently iterate over leaf
      nodes without needing to "unwrap" them from the `Node` enum.
    - `DockState::find_tab_from`/`Tree::find_tab_from` - a more generalized version of the existing `find_tab` methods
      which doesn't require the tab type to implement `PartialEq`.
    - New type `LeafNode` which contains leaf node data and has the following methods:
        * `new`,
        * `set_active_tab`,
        * `set_rect`,
        * `rect`,
        * `len`,
        * `is_empty`,
        * `tabs`,
        * `tabs_mut`,
        * `append_tab`,
        * `insert_tab`,
        * `remove_tab`,
        * `retain_tabs`,
        * `active_focused`.
    - New type `SplitNode` which contains data about node splits and has the following methods:
        * `new`,
        * `set_rect`,
        * `rect`.
    - `Node::get_leaf`/`Node::get_leaf_mut` - an alternative way of trying to access leaf data in a node.
- `TabBarStyle` now has two new fields: `inner_margin` and
  `spacing`. ([#270](https://github.com/Adanos020/egui_dock/pull/270))

### Fixed

- `DockState::retain_tabs` no longer deletes the main surface if it ends up empty
  ([#277](https://github.com/Adanos020/egui_dock/pull/277)).
- From [#275](https://github.com/Adanos020/egui_dock/pull/275):
    - `{DockState,Tree}::remove_leaf` now removes unused empty node`s at the back of the tree.
    - `{DockState,Tree}::retain_tabs` no longer deletes leaf nodes it shouldn't delete.

## egui_dock 0.16.0 - 2025-02-07

### Breaking changes

- Upgraded to egui 0.31.

## egui_dock 0.15.0 - 2024-12-28

### Changed

- From ([#237](https://github.com/Adanos020/egui_dock/pull/237)):
    - Each leaf can now be collapsed / closed individually. They are introduced as additional tab bar controls.
    - Undocked windows are now more compact. The original undocked window controls are now accessible as "secondary
      buttons" from the tab bar.
        - By default, the secondary buttons are activated from primary buttons either by holding the <kbd>Shift</kbd>
          key while clicking on them, or from a context menu by right-clicking them.
    - A number of tooltip hints are on by default as guides to the new behavior, but they can be disabled.
    - There has been an overhaul to the internal codebase to support the new features.

### Added

- From ([#237](https://github.com/Adanos020/egui_dock/pull/237)):
    - `DockArea::show_leaf_close_all_buttons` – shows a close all button which closes all open tabs in a leaf.
    - `DockArea::show_leaf_collapse_buttons` – shows a collapsing button which collapses a leaf (no longer collapsing a
      window).
    - `DockArea::show_secondary_button_hint` – sets whether tooltip hints are shown for secondary buttons on tab bars.
    - `DockArea::show_leaf_collapse_buttons` – shows a collapsing button which collapses a leaf (no longer collapsing a
      window).
    - `DockArea::secondary_button_on_modifier` – sets whether the secondary buttons on tab bars are activated by the
      modifier key.
    - `DockArea::secondary_button_context_menu` – sets whether the secondary buttons on tab bars are activated from a
      context value by right-clicking primary buttons.
    - Added the following translations:
        - `LeafTranslations::close_all_button`
        - `LeafTranslations::close_all_button_menu_hint`
        - `LeafTranslations::close_all_button_modifier_hint`
        - `LeafTranslations::close_all_button_modifier_menu_hint`
        - `LeafTranslations::close_all_button_disabled_tooltip`
        - `LeafTranslations::minimize_button`
        - `LeafTranslations::minimize_button_menu_hint`
        - `LeafTranslations::minimize_button_modifier_hint`
        - `LeafTranslations::minimize_button_modifier_menu_hint`
    - `Node::is_collapsed` – returns whether the `Node` is collapsed.
    - `Node::collapsed_leaf_count` – returns the number of collapsed layers of leaf subnodes.
    - `Node::set_collapsed` – set the collapsing state of the `Node`.
    - `Node::set_collapsed_leaf_count` – sets the number of collapsed layers of leaf subnodes.
    - `WindowState::minimized` field – records whether a window is minimized.
    - `WindowState::expanded_height` field – records the height of the window before it was fully collapsed.
    - Added style configuration for the two buttons:
        - `ButtonsStyle::{close_all_tabs, collapse_tabs, minimize_window}_color`
        - `ButtonsStyle::{close_all_tabs, collapse_tabs, minimize_window}_active_color`
        - `ButtonsStyle::{close_all_tabs, collapse_tabs, minimize_window}_bg_fill`
        - `ButtonsStyle::{close_all_tabs, collapse_tabs, minimize_window}_border_color`
        - `ButtonsStyle::close_all_tabs_disabled_color`
        - `Style::TAB_CLOSE_ALL_BUTTON_SIZE`
        - `Style::TAB_CLOSE_ALL_SIZE`
        - `Style::TAB_COLLAPSE_BUTTON_SIZE`
        - `Style::TAB_COLLAPSE_ARROW_SIZE`
        - `Style::TAB_EXPAND_BUTTON_SIZE`
        - `Style::TAB_EXPAND_ARROW_SIZE`

### Breaking changes

- From ([#237](https://github.com/Adanos020/egui_dock/pull/237)):
    - Renamed `Translations::WindowTranslations` to `Translations::LeafTranslations`.
    - Renamed `WindowTranslations::close_button_tooltip` to `LeafTranslations::close_button_disabled_tooltip`.
    - `Translations::LeafTranslations` now requires more fields to be constructed (see **Added** section).
- Upgraded to egui 0.30.

### Deprecated

- From ([#237](https://github.com/Adanos020/egui_dock/pull/237)):
    - `DockArea::show_window_close_buttons` – no longer has any effect; consider using
      `DockArea::show_leaf_close_all_buttons`
      instead.
    - `DockArea::show_window_collapse_buttons` – no longer has any effect; consider using
      `DockArea::show_leaf_collapse_buttons`
      instead.

## 0.14.0 - 2024-09-02

### Breaking changes

- Upgraded to egui 0.29.

### Changed

- `{DockState,Surface,Tree,Node}::{filter_map_tabs,map_tabs,filter_tabs,retain_tabs}` no longer require the predicate to
  implement `Clone`. ([#241](https://github.com/Adanos020/egui_dock/pull/241))

## 0.13.0 - 2024-07-03

### Breaking changes

- Upgraded to egui 0.28.
- Changed MSRV to 1.76.

## 0.12.0 - 2024-04-05

### Breaking changes

- Upgraded to egui 0.27.

### Changed

- All `Style` structs are now serializable with `serde`. ([#227](https://github.com/Adanos020/egui_dock/pull/227))

### Fixed

- Dragging tabs around should no longer cause the `DockArea` to resize a tiny bit on every frame.
- Dragged tabs should now always follow the mouse exactly.
- Button overlay now correctly renders split buttons when allowed splits are either `LeftRightOnly` or `TopBottomOnly`.

## 0.11.4 - 2024-03-11

### Fixed

- Tab body's background is now rounded with the value of `TabBodyStyle::rounding`.
  ([#232](https://github.com/Adanos020/egui_dock/pull/232))

## 0.11.3 - 2024-03-07

### Fixed

- `filter_map_tabs` sometimes deleting nodes when it shouldn't.
  ([#230](https://github.com/Adanos020/egui_dock/pull/230))

## 0.11.2 - 2024-02-16

### Fixed

From [#225](https://github.com/Adanos020/egui_dock/pull/225):

- Tabs now always appear at the pointer position while being dragged.
- Retaining tabs no longer breaks the binary tree leading to a panic.
- Filtering tabs no longer leaves some leaves empty and now correctly rearranges the tree.

## 0.11.1 - 2024-02-09

### Fixed

- Bug where tabs couldn't be re-docked onto the main surface if it's empty.
  ([#222](https://github.com/Adanos020/egui_dock/pull/222))

## 0.11.0 - 2024-02-06

### Added

- `filter_map_tabs`, `filter_tabs`, and `retain_tabs`. ([#217](https://github.com/Adanos020/egui_dock/pull/217))

### Breaking changes

- Upgraded to egui 0.26.

## 0.10.0 - 2024-01-09

### Added

- From ([#211](https://github.com/Adanos020/egui_dock/pull/211)):
    - Tabs, the close tab buttons and the add tab buttons are now focusable with the keyboard and interactable with the
      enter key and space bar.
    - Separators are now focusable with the keyboard and movable using the arrow keys while control or shift is held.
    - `TabStyle::active_with_kb_focus`, `TabStyle::inactive_with_kb_focus` and `TabStyle::focused_with_kb_focus` for
      style of tabs that are focused with the keyboard.
- Missing translation for the tooltip showing when you hover on a grayed out window close button.
  ([#216](https://github.com/Adanos020/egui_dock/pull/216))

### Fixed

- Widgets inside tabs are now focusable with the tab key on the keyboard.
  ([#211](https://github.com/Adanos020/egui_dock/pull/211))

### Breaking changes

- Upgraded to egui 0.25
- Replaced `Default` implementations for `{TabContextMenu,Window,}Translations` with associated functions called
  `english`. ([#216](https://github.com/Adanos020/egui_dock/pull/216))

## 0.9.1 - 2023-12-10

### Fixed

- Fix crash after calling `DockState::remove_tab`. ([#208](https://github.com/Adanos020/egui_dock/pull/208))

## 0.9.0 - 2023-11-23

### Added

- `DockArea::surfaces_count`
- `DockArea::iter_surfaces[_mut]`
- `DockArea::iter_all_tabs[_mut]`
- `DockArea::iter_all_nodes[_mut]`
- `Node::iter_tabs[_mut]`
- `Surface::iter_nodes[_mut]`
- `Surface::iter_all_tabs[_mut]`

### Breaking changes

- Upgraded to egui 0.24.
- Removed the deprecated `DockState::iter`.

### Deprecated

- `DockState::iter_nodes` – use `iter_all_nodes` instead.
- `DockState::iter_main_surface_nodes[_mut]` – use `dock_state.main_surface().iter()` (and corresponding `mut` versions)
  instead.

## 0.8.2 - 2023-11-02

### Fixed

- Deserializing `WindowState` no longer crashes when `screen_rect` contains any `f32::INFINITY` values. Make sure to fix
  your last serialized app state by setting `screen_rect: null`.
  ([#198](https://github.com/Adanos020/egui_dock/pull/198))

## 0.8.1 - 2023-10-04

### Fixed

- The tab bar no longer remains empty after it ends up having 0 width in any way.
  ([#191](https://github.com/Adanos020/egui_dock/pull/191))

## 0.8.0 - 2023-09-28

### Breaking changes

- Upgraded `egui` to version 0.23.
- Updated MSRV to Rust 1.70.

### Improvements

- Revised documentation for `TabViewer`.

## 0.7.3 - 2023-09-22

### Fixed

- The "Eject" button is not available on tabs which are disallowed in windows.
  ([#188](https://github.com/Adanos020/egui_dock/pull/188))

## 0.7.2 - 2023-09-20

### Fixed

- `TabViewer::clear_background` now works as intended. ([#185](https://github.com/Adanos020/egui_dock/pull/185))

## 0.7.1 - 2023-09-18

### Fixed

- (Breaking) Renamed `OverlayStyle::selection_storke_width` to `OverlayStyle::selection_stroke_width`.

## 0.7.0 - 2023-09-18

This is the biggest update so far, introducing the long awaited undocking feature: tabs can now be dragged out into new
egui windows. Massive thanks to [Vickerinox](https://github.com/Vickerinox) for implementing it!

This update also includes an overhaul of the documentation, aiming to not only be more readable and correct, but also
provide a guide of how to use the library.

### Changed

- Adjusted the styling of tabs to closer follow the egui default styling.
  ([#139](https://github.com/Adanos020/egui_dock/pull/139))
- Double-clicking on a separator resets the size of both adjacent nodes.
  ([#146](https://github.com/Adanos020/egui_dock/pull/146))
- Tabs can now only be dragged with the primary pointer button (e.g. left mouse button).
  ([#177](https://github.com/Adanos020/egui_dock/pull/177))

### Fixed

- Correctly draw a border around a dock area using the `Style::border`
  property. ([#139](https://github.com/Adanos020/egui_dock/pull/139))
- Non-closable tabs now cannot be closed by clicking with the middle mouse button.
  ([9cdef8c](https://github.com/Adanos020/egui_dock/pull/149/commits/9cdef8cb77e73ef7a065d1313f7fb8feae0253b4))
- Dragging tabs around now works on touchscreens. ([#180](https://github.com/Adanos020/egui_dock/pull/180))

### Added

- From [#139](https://github.com/Adanos020/egui_dock/pull/139):
    - `Style::main_surface_border_rounding` for the rounding of the dock area border.
    - `TabStyle::active` for the active style of a tab.
    - `TabStyle::inactive` for the inactive style of a tab.
    - `TabStyle::focused` for the focused style of a tab.
    - `TabStyle::hovered` for the hovered style of a tab.
    - `TabStyle::tab_body` for styling the body of the tab including background color, stroke color, rounding and inner
      margin.
    - `TabStyle::minimum_width` to set the minimum width of the tab.
    - `TabInteractionStyle` to style the active/inactive/focused/hovered states of a tab.
- `AllowedSplits` enum which lets you choose in which directions a `DockArea` can be split.
  ([#145](https://github.com/Adanos020/egui_dock/pull/145))
- From [#149](https://github.com/Adanos020/egui_dock/pull/149):
    - `DockState<Tab>` containing the entire state of the tab hierarchies stored in a collection of `Surfaces`.
    - `Surface<Tab>` enum which represents an area (e.g. a window) with its own `Tree<Tab>`.
    - `SurfaceIndex` to identify a `Surface` stored in the `DockState`.
    - `Split::is_tob_bottom` and `Split::is_left_right`.
    - `TabInsert` which replaces current `TabDestination` (see breaking changes).
    - `impl From<(SurfaceIndex, NodeIndex, TabInsert)> for TabDestination`.
    - `impl From<SurfaceIndex> for TabDestination`.
    - `TabDestination::is_window` (see breaking changes).
    - `Tree::root_node` and `Tree::root_node_mut`.
    - `Node::rect` returning the `Rect` occupied by the node.
    - `Node::tabs` and `Node::tabs_mut` returning an optional slice of tabs if the node is a leaf.
    - `WindowState` representing the current state of a `Surface::Window` and allowing you to manipulate the window.
    - `OverlayStyle` (stored as `Style::overlay`) and `OverlayFeel`: they specify the look and feel of the drag-and-drop
      overlay.
    - `OverlayType` letting you choose if the overlay should be the new icon buttons or the old highlighted rectangles.
    - `LeafHighlighting` specifying how a currently hovered leaf should be highlighted.
    - `DockArea::window_bounds` setting the area which windows are constrained by.
    - `DockArea::show_window_close_buttons` setting determining if windows should have a close button or not.
    - `DockArea::show_window_collapse_buttons` setting determining if windows should have a collapse button or not.
    - `TabViewer::allowed_in_windows` specifying if a given tab can be shown in a window.
- `TabViewer::closable` lets individual tabs be closable or not.
  ([#150](https://github.com/Adanos020/egui_dock/pull/150))
- `TabViewer::scroll_bars` specifying if horizontal and vertical scrolling is enabled for given tab – replaces
  `DockArea::scroll_area_in_tabs` (see breaking changes). ([#160](https://github.com/Adanos020/egui_dock/pull/160))
- `Translations` specifying what text will be displayed in some parts of the `DockingArea`, e.g. the tab context menus
  (defined in `TabContextMenuTranslations`). ([#178](https://github.com/Adanos020/egui_dock/pull/178))

### Breaking changes

- From [#139](https://github.com/Adanos020/egui_dock/pull/139):
    - Moved `TabStyle::inner_margin` to `TabBodyStyle::inner_margin`.
    - Moved `TabStyle::fill_tab_bar` to `TabBarStyle::fill_tab_bar`.
    - Moved `TabStyle::outline_color` to `TabInteractionStyle::outline_color`.
    - Moved `TabStyle::rounding` to `TabInteractionStyle::rounding`.
    - Moved `TabStyle::bg_fill` to `TabInteractionStyle::bg_fill`.
    - Moved `TabStyle::text_color_unfocused` to `TabStyle::inactive.text_color`.
    - Moved `TabStyle::text_color_active_focused` to `TabStyle::focused.text_color`.
    - Moved `TabStyle::text_color_active_unfocused` to `TabStyle::active.text_color`.
    - Renamed `Style::tabs` to `Style::tab`.
    - Removed `TabStyle::text_color_focused`. This style was practically never reachable.
- From [#149](https://github.com/Adanos020/egui_dock/pull/149):
    - `TabDestination` now specifies if a tab will be moved to a `Window`, a `Node`, or an `EmptySurface`. Its original
      purpose is now served by `TabInsert`.
    - `Tree::split` now panics if supplied `fraction` is not in range 0..=1.
    - Moved `Tree::move_tab` to `DockState::move_tab`.
    - Renamed `Style::border` to `Style::main_surface_border_stroke`.
    - Moved `Style::selection_color` to `OverlayStyle::selection_color`.
    - `DockArea::new` now takes in a `DockState` instead of a `Tree`.
- Removed `DockArea::scroll_area_in_tabs` – override `TabViewer::scroll_bars`
  instead. ([#160](https://github.com/Adanos020/egui_dock/pull/160))
- Methods `TabViewer::{context_menu,on_add,add_popup}` now take in an additional `SurfaceIndex`
  parameter. ([#167](https://github.com/Adanos020/egui_dock/pull/167))

## 0.6.3 - 2023-06-16

### Fixed

- Made the `DockArea` always allocate an area ([#143](https://github.com/Adanos020/egui_dock/pull/143))

## 0.6.2 - 2023-06-09

### Fixed

- Make the `max_size` of `tabbar_inner_rect` finite ([#141](https://github.com/Adanos020/egui_dock/pull/141))

## 0.6.1 - 2023-05-29

### Fixed

- Ensure rect size are calculated before drawing node bodies ([#134](https://github.com/Adanos020/egui_dock/pull/134))

## 0.6.0 - 2023-05-24

### Added

- `TabViewer::tab_style_override` that lets you define a custom `TabsStyle` for an individual tab
  ([99333b0](https://github.com/Adanos020/egui_dock/commit/99333b093d307181c288b3e134379cfe47647a7c))
- `ButtonsStyle::add_tab_border_color` for the `+` button's left border
  ([99333b0](https://github.com/Adanos020/egui_dock/commit/99333b093d307181c288b3e134379cfe47647a7c))
- `TabBarStyle::rounding` for rounding of the tab bar, independent from tab rounding
  ([99333b0](https://github.com/Adanos020/egui_dock/commit/99333b093d307181c288b3e134379cfe47647a7c))
- Separate `from_egui` methods for `ButtonsStyle`, `SeparatorStyle`, `TabBarStyle`, and `TabStyle`
  ([a660497](https://github.com/Adanos020/egui_dock/commit/a660497b21651dd9920665bf50d8fc9e75d0e1e0))

### Breaking changes

- Upgraded `egui` to version 0.22
  ([c2e8fee](https://github.com/Adanos020/egui_dock/commit/c2e8feeb7713e2b2d2f0fa1b13a46732f9c6df62))
- Renamed `TabsStyle`
  to `TabStyle` ([89f3248](https://github.com/Adanos020/egui_dock/commit/89f32487a9e1fe8dee92f1fbdc296a2d460c0909))
-

Removed
`StyleBuilder` ([9a9b275](https://github.com/Adanos020/egui_dock/commit/9a9b2750cd290bebcc4088761249e02102cb0ce7))

- Removed `TabViewer::inner_margin_override` – no deprecation as it's in direct conflict with
  `TabViewer::tab_style_override`
  ([99333b0](https://github.com/Adanos020/egui_dock/commit/99333b093d307181c288b3e134379cfe47647a7c))
- Moved `Style::default_inner_margin`
  to
  `TabsStyle::inner_margin`
  ([78ecf3a](https://github.com/Adanos020/egui_dock/commit/78ecf3a175ffb960724f328274682dfded800e0f))
- Moved `TabStyle::hline_color`
  to
  `TabBarStyle::hline_color`
  ([99333b0](https://github.com/Adanos020/egui_dock/commit/99333b093d307181c288b3e134379cfe47647a7c))

## 0.5.2 - 2023-06-04

### Fixed

- Ensure rect size are calculated before drawing node bodies ([#134](https://github.com/Adanos020/egui_dock/pull/134))

## 0.5.1 - 2023-05-20

## Fixed

- Ensure close button can be scrolled to when tab bar is small ([#129](https://github.com/Adanos020/egui_dock/pull/129))

### Added

- `SeparatorStyle::extra_interact_width` option that adds "logical" width to separators so that they are easier to grab
  ([#128](https://github.com/Adanos020/egui_dock/pull/128))

## 0.5.0 - 2023-04-22

### Fixed

- Ensure `Tab` have a stable `egui::Id` when moved ([#121](https://github.com/Adanos020/egui_dock/pull/121))
- Don't display the "grab" cursor icon on tabs when hovered and the `draggable_tabs` flag is unset
  ([#123](https://github.com/Adanos020/egui_dock/pull/123))

### Added

- `Tree::move_tab` method that allows moving a tab from one node to the other
  ([#115](https://github.com/Adanos020/egui_dock/pull/107))
- `Tree::remove_leaf` method that deletes a selected leaf node ([#115](https://github.com/Adanos020/egui_dock/pull/107))
- New methods in `DockArea` ([#115](https://github.com/Adanos020/egui_dock/pull/115))
    - `show_add_popup`
    - `show_add_buttons`
    - `show_close_buttons`
    - `draggable_tabs`
    - `tab_context_menus`
    - `scroll_area_in_tabs`
    - `show_tab_name_on_hover`
- Make tabs scrollable when they overflow ([#116](https://github.com/Adanos020/egui_dock/pull/116))
- `TabViewer::id` method that allows specifying a custom id for each tab
  ([#121](https://github.com/Adanos020/egui_dock/pull/121))

### Breaking changes

- Removed `remove_empty_leaf` which was used for internal usage and should not be needed by users
  ([#115](https://github.com/Adanos020/egui_dock/pull/107))
- Removed `show_close_buttons` from `StyleBuilder` ([#115](https://github.com/Adanos020/egui_dock/pull/115))
- Moved the following fields from `Style` to `DockArea` ([#115](https://github.com/Adanos020/egui_dock/pull/115))
    - `show_add_popup`
    - `show_add_buttons`
    - `show_close_buttons`
    - `tabs_are_draggable` (renamed to `draggable_tabs`)
    - `show_context_menu` (renamed to `tab_context_menus`)
    - `tab_include_scrollarea` (renamed to `scroll_area_in_tabs`)
    - `tab_hover_name` (renamed to `show_tab_name_on_hover`)
- `Style` is now split up into smaller structs for maintainability and consistence with `egui::Style`
  ([#115](https://github.com/Adanos020/egui_dock/pull/115))

| Old names and locations                         | New names and locations                          |
|-------------------------------------------------|--------------------------------------------------|
| `Style::border_color` and `Style::border_width` | `Style::border` (which is now an `egui::Stroke`) |
| `Style::separator_width`                        | `Separator::width`                               |
| `Style::separator_extra`                        | `Separator::extra`                               |
| `Style::separator_color_idle`                   | `Separator::color_idle`                          |
| `Style::separator_color_hovered`                | `Separator::color_hovered`                       |
| `Style::separator_color_dragged`                | `Separator::color_dragged`                       |
| `Style::tab_bar_background_color`               | `TabBar::bg_fill`                                |
| `Style::tab_bar_height`                         | `TabBar::height`                                 |
| `Style::tab_outline_color`                      | `Tabs::outline_color`                            |
| `Style::hline_color`                            | `Tabs::hline_color`                              |
| `Style::hline_below_active_tab_name`            | `Tabs::hline_below_active_tab_name`              |
| `Style::tab_rounding`                           | `Tabs::rounding`                                 |
| `Style::tab_background_color`                   | `Tabs::bg_fill`                                  |
| `Style::tab_text_color_unfocused`               | `Tabs::text_color_unfocused`                     |
| `Style::tab_text_color_focused`                 | `Tabs::text_color_focused`                       |
| `Style::tab_text_color_active_unfocused`        | `Tabs::text_color_active_unfocused`              |
| `Style::tab_text_color_active_focused`          | `Tabs::text_color_active_focused`                |
| `Style::expand_tabs`                            | `Tabs::fill_tab_bar`                             |
| `Style::close_tab_color`                        | `Buttons::close_tab_color`                       |
| `Style::close_tab_active_color`                 | `Buttons::close_tab_active_color`                |
| `Style::close_tab_background_color`             | `Buttons::close_tab_bg_fill`                     |
| `Style::add_tab_align`                          | `Buttons::add_tab_align`                         |
| `Style::add_tab_color`                          | `Buttons::add_tab_color`                         |
| `Style::add_tab_active_color`                   | `Buttons::add_tab_active_color`                  |
| `Style::add_tab_background_color`               | `Buttons::add_tab_bg_fill`                       |

### Deprecated

- `StyleBuilder`

## 0.4.2 - 2023-03-17

### Fixed

- `TabViewer::clear_background` works again ([#110](https://github.com/Adanos020/egui_dock/pull/110))

## 0.4.1 - 2023-03-14

### Fixed

- Light mode now works in tabs
  ([528b892](https://github.com/Adanos020/egui_dock/commit/528b89245928d055dabb00cd9001c22d275f789b))
- `DockArea::show_inside` no longer obscures previously added elements
  ([#102](https://github.com/Adanos020/egui_dock/pull/102))
- Splitter drag now behaves like egui `DragValue` ([#103](https://github.com/Adanos020/egui_dock/pull/103))

## 0.4.0 - 2023-02-09

### Added

- Added `TabViewer::on_tab_button` ([#93](https://github.com/Adanos020/egui_dock/pull/93)).

### Breaking changes

- Updated to egui 0.21
- Deleted `dynamic_tab` which was deprecated in 0.3.0

### Fixed

- Make splitter drag behave like egui `DragValue` ([#103](https://github.com/Adanos020/egui_dock/pull/103))

## 0.3.1 - 2022-12-21

### Added

- `Style` now includes an option to change the tab's height - `tab_bar_height`.
  ([#62](https://github.com/Adanos020/egui_dock/pull/62))
- Implemented the `std::fmt::Debug` trait on all exported types. ([#84](https://github.com/Adanos020/egui_dock/pull/84))

### Fixed

- Errors in the README

## 0.3.0 - 2022-12-10

### Added

- `TabViewer::clear_background` method that returns if current tab's background should be cleared.
  ([#35](https://github.com/Adanos020/egui_dock/pull/35))
- You can now close tabs with middle mouse button if `Style::show_close_buttons` is true.
  ([#34](https://github.com/Adanos020/egui_dock/pull/34))
- Option to disable dragging tabs.
- New option `expand_tabs` in `Style` and `StyleBuiler` causes tab titles to expand to match the width of their tab
  bars.
- `StyleBuilder::from_egui`. ([#40](https://github.com/Adanos020/egui_dock/pull/40))
- `Tree::find_active_focused`. ([#40](https://github.com/Adanos020/egui_dock/pull/40))
- Added `context_menu` into `TabViewer`. ([#46](https://github.com/Adanos020/egui_dock/pull/46))
- The `ScrollArea` inside a tab is now optional via `Style`. ([#49](https://github.com/Adanos020/egui_dock/pull/49))
- `Tree::tabs`: an iterator over the tabs in a tree. ([#53](https://github.com/Adanos020/egui_dock/pull/53))
- `Style` now includes an option to show the hovered tab's name. ([#56](https://github.com/Adanos020/egui_dock/pull/56))
- `Style` now includes an option to change default inner_margin. ([#67](https://github.com/Adanos020/egui_dock/pull/67))
- The split separator now highlights on hover ([#68](https://github.com/Adanos020/egui_dock/pull/68))
- Tabs can now be removed with `Tree::remove_tab` ([#70](https://github.com/Adanos020/egui_dock/pull/70))

### Breaking changes

- Renamed `TabViewer::inner_margin`
  to `TabViewer::inner_margin_override`. ([#67](https://github.com/Adanos020/egui_dock/pull/67))
- `Style::with_separator_color` has been split into `separator_color_idle`, `separator_color_hovered`,
  `separator_color_dragged` ([#68](https://github.com/Adanos020/egui_dock/pull/68))
- Updated `egui` to 0.20.0 [#77](https://github.com/Adanos020/egui_dock/pull/77)

### Deprecated (will be deleted in the next release)

- `dynamic_tab::TabContent`
- `dynamic_tab::OnClose`
- `dynamic_tab::ForceClose`
- `dynamic_tab::TabBuilder`
- `dynamic_tab::Tab`
- `dynamic_tab::BuiltTab`
- `dynamic_tab::DynamicTree`
- `dynamic_tab::DynamicTabViewer`

## 0.2.1 - 2022-09-09

### Added

- Added opt-in `serde` feature to enable serialization of `Tree`.
- You can now change the tab text color with `Style::tab_text_color_unfocused` and `Style::tab_text_color_focused`.

### Fixed

- `Tree::push_to_first_leaf` no longer panics when used on an empty `Tree`.
- The tab text color will now follow the egui text color.

## 0.2.0 - 2022-09-04

### Added

- It is now possible to close tabs with a close button that can be shown/hidden through `Style`.
- When dragging tabs onto the tab bar if the tab will be inserted a highlighted region will show where the tab will end
  up if dropped.
- The dock will keep track of the currently focused leaf.
- Using `Tree::push_to_focused_leaf` will push the given tab to the currently active leaf.
- `StyleBuilder` for the `Style`.
- New fields in `Style:` `separator_color`, `border_color`, and `border_width` (last two for the cases when used
  `Margin`).
- `TabBuilder` for the `BuiltTab`.
- Support for all implementations of `Into<WidgetText>` in tab titles.
- Style editor in the `hello` example.
- Added `Tree::find_tab`, `TabViewer`, `DynamicTabViewer`, `DynamicTree`.
- Added a `text_editor` example.

### Changed

- If a tab is dropped onto the tab bar it will be inserted into the index that it is dropped onto.
- Now when you drag a tab it has an outline along the entire length of the edges of it.
- Bumped MSRV to `1.62`.
- `Tree` is now generic over how you want to represent a tab.

### Breaking changes

- Ui code of the dock has been moved into `DockArea` and is displayed with `DockArea::show` or `DockArea::show_inside`.
- Renamed `Style::border_size` to `Style::border_width`.
- Renamed `Style::separator_size` to `Style::separator_width`.
- Removed `Style::tab_text_color` as you can now set the tab text color of a tab by passing `RichText` for its title.
- Removed the requirement of creating your own Context type.
- Renamed `Tree::set_focused` to `Tree::set_focused_node`.
- Renamed `Node::None` to `Node::Empty`.

### Fixed

- Now selection color of the placing area for the tab isn't showing if the tab is targeted on its own node when the tab
  is the only member of this node.
- Dock vertical and horizontal separators are now displayed properly.
- Prevent Id clashes from multiple tabs being displayed at once.
- Tab content is now displayed inside a `egui::ScrollArea`, so it's now accessible in its entirety even if the tab is
  too small to fit all of it.
- Fixed an issue where some tabs couldn't be resized.
