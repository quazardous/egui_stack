//! egui_stack's integration tests, as one executable (#178): each of the
//! former binaries linked egui, eframe and egui_kittest, so one link per
//! file. One module per former file; run one alone with a test name filter,
//! for example `cargo test --test it floats::`.

#[path = "support/multi.rs"]
mod multi;

mod add_button_place;
mod auto_fold_events;
mod auto_fold_strip;
mod builtin_tab_menu;
mod bus_points;
mod collapse_restore;
mod compact_drop_zones;
mod context_menu_close;
mod control_accessibility;
mod default_painting;
mod dock_painter;
mod dock_space;
mod dock_titles;
mod docked;
mod drag_across;
mod drag_activation;
mod dragged_node;
mod dragged_tab;
mod drop_allowed;
mod drop_at_release;
mod drop_on_own_slot;
mod drop_zones;
mod escape_cancels_drag;
mod float_chrome;
mod float_close;
mod float_collapse;
mod float_dock_options;
mod float_gestures;
mod float_move_from;
mod float_resize;
mod floating_windows;
mod floats;
mod ghost_after_drop;
mod grid;
mod group_bar;
mod group_drag_handle;
mod group_ghost;
mod group_handles;
mod layout_area;
mod layout_dock;
mod layout_fold;
mod layout_layers;
mod layout_sash_same_frame;
mod layout_sashes;
mod linear_split;
mod lost_release;
mod native_floats;
mod readme_version;
mod roving_tab_focus;
mod section_painting;
mod sections;
mod separator_accessibility;
mod separator_keys;
mod split_strip_drops;
mod stale_drag;
mod strip_title;
mod strip_unfolds;
mod tab_accessibility;
mod tab_actions;
mod tab_bar_end_buttons;
mod tab_cycle;
mod tab_ghost;
mod tab_padding;
mod tab_place;
mod tool_row;
mod tool_row_painting;
mod window_state;
mod work_area;
