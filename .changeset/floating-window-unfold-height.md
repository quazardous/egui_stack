---
egui_stack: patch
---

A floating window gets back the height it had once it is unfolded (or restored from minimized): its least size no longer overrode the height kept when it was folded, so it stayed as short as its folded bands. A split window whose every leaf is folded collapses to its bands and, once a leaf is unfolded, gets back its height and its splits' shares; the height kept is saved with the layout.
