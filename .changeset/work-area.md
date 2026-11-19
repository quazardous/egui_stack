---
egui_stack: minor
---

The floats over the main window can be kept inside a work area: `FloatsArea::work_area(WorkArea)`, by default `WorkArea::window()` (the whole window, as before), or `WorkArea::new()` with regions resolved every frame: `.space()` (the space as drawn), `.zone(name)` (a layout zone, as `DockSpace::bind` last read it) and `.rect(r)`. A tab or a group released outside every dock and every region stays where it was (`Outside::Cancel`, the default). A float stays in the region it overlaps most, moves to the region under the pointer without straddling a gap, is not resized past its region, and is brought back into its region or the nearest one when that region shrinks or goes, or when a layout is restored with it outside, reported once as `FloatEvent::Moved`. Native floats ignore the work area (#148).
