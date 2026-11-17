---
egui_stack: minor
---

A zone the solver folds by itself for want of room (`Constraints::auto_fold`) is reported once in `Laid::events` by `LayoutArea`: `LayoutEvent::AutoFolded` when it folds, `LayoutEvent::AutoUnfolded` when the room comes back. Its `ZoneState::folded` is still not touched, so an application tells a zone the user folded (`Folded`, `Unfolded`) from one folded for lack of room. `Layout::set_folded` still reports no event: the application made that change itself. An application matching `LayoutEvent` already has a catch-all arm (the enum is `#[non_exhaustive]`).
