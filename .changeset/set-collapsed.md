---
egui_stack: minor
---

`DockState::set_collapsed(path, collapsed)` and `Tree::set_leaf_collapsed(node, collapsed)`
collapse or expand a tab group from code as its collapse button does, keeping the tree's
collapsed state up to date: a float whose groups are all collapsed shrinks to its tab bars and
grows back once one is expanded. `Node::set_collapsed` sets the node's flag only, and left such
a float shrunk; its docs point to them. `DockState::focus_tab` does not expand a collapsed
group (#142).
