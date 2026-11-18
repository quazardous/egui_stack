---
egui_stack: minor
---

A float over the main window no longer wears a title bar of its own above its dock: its top tab bar is its top, its move handle moves it and the close all button of its tab bars closes it, as the legacy windows looked before the demo's first page moved to `FloatsArea` in 0.28.5. `FloatsArea::title_bar(true)` brings the title bar back (the dock's title and a close button, which moves the window too); a float whose move handle is turned off keeps it, to be moved by. Native floats keep the system's decoration.
