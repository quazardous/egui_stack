/// Container for dockable tabs.
pub mod dock_area;

/// Trait for tab-viewing types.
pub mod tab_viewer;

pub use dock_area::{AllowedSplits, DockArea, DraggedTab, Labels};
pub use tab_viewer::TabViewer;
