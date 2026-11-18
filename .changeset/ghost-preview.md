---
egui_stack: minor
---

`TabViewer::ghost_size` and `TabViewer::ghost_body` let an application size the ghost of a dragged tab or group and paint its body, a live preview of the tab's content under the ghost's tab bar for instance (a group shows its active tab). By default neither changes anything; a body the application paints replaces the painter's plain hint, inside the same frame and shadow.
