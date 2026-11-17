---
egui_stack: major
---

With `OverlayType::Zones`, the drop zones of a leaf leave its tab bar out: `OverlayFeel` has two new fields, `zone_tab_bar_stack_band`, a band right under the tab bar where a dropped tab joins the leaf's tabs as over the tab bar (`8.0` points by default), and `zone_edges_below_tab_bar`, which lays the edge bands on the leaf's body below the tab bar and that band (`true` by default) instead of on the whole leaf, where in a compact leaf the top band was mostly hidden under the tab bar, leaving a strip under it that split the leaf. `DropZone::in_leaf` picks a zone the way the dock does. `zone_edges_below_tab_bar: false` with `zone_tab_bar_stack_band: 0.0` gives the former geometry back. Breaking for code that builds `OverlayFeel` with a struct literal naming every field: add the new field, or end the literal with `..Default::default()`.
