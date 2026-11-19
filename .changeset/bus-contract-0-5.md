---
egui_stack: major
---

The `bus` feature now uses bus-contract 0.5 (`egui_stack::bus::bus_contract` re-exports it): each point's `Point::ID` is a checked `PointId` instead of a `&'static str`. The point names are unchanged (`stack.dock.drop`, `stack.layout.changed`, `stack.window.detached`, `stack.window.close`, `stack.window.closed`, `stack.window.repaired`), so listeners keep matching them. Migration: move the application's bus to bus-contract 0.5 (and bus-kernel 0.3 if it uses it) following bus-contract's MIGRATION-0.5.md; write `&P::ID` or `P::ID.as_str()` where a `&str` is needed, and give every listener id a prefix (`app.autosave`, not `autosave`), which a 0.5 bus now requires.
