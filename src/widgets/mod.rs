/// Container for dockable tabs.
pub mod dock_area;

/// Painting the parts of a dock: tabs, tab bars, separators, the drop overlay.
pub mod painter;

/// Trait for tab-viewing types.
pub mod tab_viewer;

pub use dock_area::{
    AllowedSplits, DockArea, DragPayload, DraggedNode, DraggedTab, DropZone, GroupHandles, Labels,
    SpaceDrag, TabFocus,
};
pub use painter::{
    DockPainter, DraggedGroupMore, DraggedGroupPaint, DraggedGroupTab, DropButtonPaint, DropPaint,
    DropRefusal, DropTargetPaint, PartState, RestoreBandPaint, SectionBandPaint,
    SectionBoundaryPaint, SeparatorAxis, SeparatorPaint, StylePainter, TabBarGripPaint,
    TabBarPaint, TabBarTailPaint, TabBodyPaint, TabClose, TabMeasure, TabPaint, ToolItemPaint,
    ToolMorePaint, ToolSeparatorPaint, WindowOutlinePaint,
};
pub use tab_viewer::{DockDrop, TabViewer};
