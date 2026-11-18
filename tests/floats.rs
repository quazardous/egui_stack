//! The float model: places, their repair onto the screens there are, the
//! layout's floats and closed places, tab origins, and their persistence.

use egui::{Id, Rect, pos2, vec2};
use egui_stack::layout::{
    Content, Dir, Float, FloatHost, Layout, Place, Repaired, Screen, ScreenGone, Zone,
};
use egui_stack::{DockRules, DockSpace, Docked, NodeIndex, TabIndex, TabPath};

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect::from_min_size(pos2(x, y), vec2(width, height))
}

/// A 1920×1080 screen at scale 1, at the desktop's origin.
fn left() -> Screen {
    Screen::new("left", rect(0.0, 0.0, 1920.0, 1080.0), 1.0)
}

/// A 3840×2160 screen at scale 2, right of `left`.
fn right() -> Screen {
    Screen::new("right", rect(1920.0, 0.0, 3840.0, 2160.0), 2.0)
}

fn moved(repaired: Repaired) -> Place {
    match repaired {
        Repaired::Moved(place) => place,
        other => panic!("expected a new place, got {other:?}"),
    }
}

#[test]
fn a_place_on_its_screen_is_left_alone() {
    let place = Place::on(&left(), rect(100.0, 100.0, 400.0, 300.0));
    let repaired = place.repair(&[left(), right()], ScreenGone::default());
    assert_eq!(repaired, Repaired::Unchanged);
}

#[test]
fn no_screen_known_changes_nothing() {
    // A system that hides positions: never invent one.
    let place = Place::on(&left(), rect(-5000.0, -5000.0, 400.0, 300.0));
    assert_eq!(place.repair(&[], ScreenGone::MoveOnto), Repaired::Unchanged);
}

#[test]
fn a_window_partly_off_screen_with_its_title_bar_on_stays() {
    // Half of it past the right edge, its title bar's left part on screen.
    let place = Place::on(&left(), rect(1700.0, 200.0, 400.0, 300.0));
    assert_eq!(
        place.repair(&[left()], ScreenGone::default()),
        Repaired::Unchanged
    );
    // Lower than the screen's bottom but for its title bar: still reachable.
    let place = Place::on(&left(), rect(300.0, 1040.0, 400.0, 300.0));
    assert_eq!(
        place.repair(&[left()], ScreenGone::default()),
        Repaired::Unchanged
    );
}

#[test]
fn a_title_bar_off_every_screen_comes_back_as_little_as_needed() {
    // Above the screen: down to its top edge, x kept.
    let place = Place::on(&left(), rect(300.0, -200.0, 400.0, 300.0));
    let place = moved(place.repair(&[left()], ScreenGone::default()));
    assert_eq!(place.rect, rect(300.0, 0.0, 400.0, 300.0));
    // Far right: back until a grab's width of its title bar is on screen.
    let place = Place::on(&left(), rect(1900.0, 200.0, 400.0, 300.0));
    let place = moved(place.repair(&[left()], ScreenGone::default()));
    assert_eq!(place.rect, rect(1920.0 - Place::GRAB, 200.0, 400.0, 300.0));
}

#[test]
fn a_title_bar_on_the_next_screen_is_reachable() {
    // Past the left screen's right edge, but on the right screen.
    let place = Place::on(&left(), rect(1950.0, 100.0, 400.0, 300.0));
    assert_eq!(
        place.repair(&[left(), right()], ScreenGone::default()),
        Repaired::Unchanged
    );
}

#[test]
fn a_window_larger_than_a_smaller_screen_shrinks_inside_it() {
    // Saved on a 2560×1440 screen, which is now 1280×720.
    let small = Screen::new("left", rect(0.0, 0.0, 1280.0, 720.0), 1.0);
    let place = Place::on(&left(), rect(200.0, 100.0, 1600.0, 900.0));
    let place = moved(place.repair(&[small], ScreenGone::default()));
    assert_eq!(place.rect, rect(0.0, 0.0, 1280.0, 720.0));
    // Only the side that does not fit shrinks; it is then moved inside.
    let small = Screen::new("left", rect(0.0, 0.0, 1280.0, 1080.0), 1.0);
    let place = Place::on(&left(), rect(600.0, 100.0, 1600.0, 300.0));
    let place = moved(place.repair(&[small], ScreenGone::default()));
    assert_eq!(place.rect, rect(0.0, 100.0, 1280.0, 300.0));
}

#[test]
fn a_window_whose_screen_is_gone_moves_onto_the_first_one() {
    let place = Place::on(&right(), rect(500.0, 400.0, 400.0, 300.0));
    let place = moved(place.repair(&[left()], ScreenGone::MoveOnto));
    assert_eq!(place.screen.as_deref(), Some("left"));
    assert_eq!(place.scale, 1.0);
    // Its offset kept in physical pixels (×2), then kept inside the screen.
    assert_eq!(place.rect, rect(1000.0, 780.0, 400.0, 300.0));
}

#[test]
fn a_window_whose_screen_is_gone_may_dock_back() {
    let place = Place::on(&right(), rect(500.0, 400.0, 400.0, 300.0));
    assert_eq!(
        place.repair(&[left()], ScreenGone::DockBack),
        Repaired::DockBack
    );
}

#[test]
fn a_place_with_no_screen_is_read_on_the_first_one() {
    // A migrated window: no screen, never docked back.
    let place = Place::new(rect(40.0, 60.0, 320.0, 240.0));
    let place = moved(place.repair(&[right()], ScreenGone::DockBack));
    assert_eq!(place.screen.as_deref(), Some("right"));
    assert_eq!(place.scale, 2.0);
    assert_eq!(place.rect, rect(40.0, 60.0, 320.0, 240.0));
}

#[test]
fn a_scale_change_reprojects_the_offset_and_keeps_the_size() {
    // Saved on the right screen at scale 2; the screen is now at scale 1.
    let place = Place::on(&right(), rect(100.0, 100.0, 400.0, 300.0));
    let now = Screen::new("right", rect(1920.0, 0.0, 3840.0, 2160.0), 1.0);
    let place = moved(place.repair(&[left(), now], ScreenGone::default()));
    assert_eq!(place.scale, 1.0);
    assert_eq!(place.rect, rect(200.0, 200.0, 400.0, 300.0));
}

#[test]
fn screens_of_different_scales_neither_overlap_nor_leave_gaps() {
    let screens = [left(), right()];
    // A window at the right screen's corner starts where the left one ends.
    let on_right = Place::on(&right(), rect(0.0, 0.0, 400.0, 300.0));
    let physical = on_right.physical(&right());
    assert_eq!(physical, rect(1920.0, 0.0, 800.0, 600.0));
    // A window against the left screen's right edge ends there.
    let on_left = Place::on(&left(), rect(1520.0, 0.0, 400.0, 300.0));
    let physical_left = on_left.physical(&left());
    assert_eq!(physical_left.max.x, physical.min.x);
    assert!(!physical_left.intersects(physical.shrink(0.5)));
    // From physical pixels back to a place, on the screen holding its centre.
    assert_eq!(Place::from_physical(physical, &screens), Some(on_right));
    assert_eq!(Place::from_physical(physical_left, &screens), Some(on_left));
    assert_eq!(
        Place::from_physical(rect(-900.0, -900.0, 100.0, 100.0), &screens),
        None
    );
}

fn layout() -> Layout {
    Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![Zone::leaf("centre", Content::Dock("centre".into()))],
    ))
}

fn float(key: &str, at: Rect) -> Float {
    Float::dock(key, key, FloatHost::Native, Place::on(&left(), at))
}

#[test]
fn a_closed_float_reopens_where_it_was() {
    let mut layout = layout();
    layout.open_float(float("log", rect(100.0, 100.0, 300.0, 200.0)));
    layout.float_mut("log").unwrap().place.rect = rect(700.0, 400.0, 300.0, 200.0);
    let closed = layout.close_float("log").unwrap();
    assert!(layout.float("log").is_none());
    assert_eq!(layout.closed_place("log"), Some(&closed.place));
    // Opened again at a default place: it takes its old one.
    let reopened = layout.open_float(float("log", rect(0.0, 0.0, 300.0, 200.0)));
    assert_eq!(reopened.place.rect, rect(700.0, 400.0, 300.0, 200.0));
    assert!(layout.closed_place("log").is_none());
}

#[test]
fn floats_open_in_front_and_rise() {
    let mut layout = layout();
    layout.open_float(float("a", rect(0.0, 0.0, 100.0, 100.0)));
    layout.open_float(float("b", rect(0.0, 0.0, 100.0, 100.0)));
    let order = |layout: &Layout| -> Vec<String> {
        layout
            .floats_back_to_front()
            .iter()
            .map(|float| float.key.clone())
            .collect()
    };
    assert_eq!(order(&layout), ["a", "b"]);
    assert!(layout.raise_float("a"));
    assert_eq!(order(&layout), ["b", "a"]);
    assert!(!layout.raise_float("missing"));
}

#[test]
fn repairing_the_floats_docks_back_those_whose_screen_is_gone() {
    let mut layout = layout();
    layout.open_float(Float::dock(
        "far",
        "far",
        FloatHost::Native,
        Place::on(&right(), rect(10.0, 10.0, 100.0, 100.0)),
    ));
    layout.open_float(float("near", rect(10.0, -50.0, 100.0, 100.0)));
    let repair = layout.repair_floats(&[left()], ScreenGone::DockBack);
    assert_eq!(repair.moved, ["near"]);
    assert_eq!(layout.float("near").unwrap().place.rect.min.y, 0.0);
    assert_eq!(repair.docked_back.len(), 1);
    assert_eq!(repair.docked_back[0].key, "far");
    assert!(layout.float("far").is_none());
    assert!(layout.closed_place("far").is_some());
}

#[test]
fn reconcile_keeps_the_users_floats_and_adds_the_codes_new_ones() {
    let mut code = layout();
    code.open_float(float("palette", rect(0.0, 0.0, 100.0, 100.0)));
    // Saved before floats existed: it gets the code's.
    let mut loaded = layout();
    loaded.reconcile(&code);
    assert!(loaded.float("palette").is_some());
    // A float the user detached, unknown to the code, stays; one the user
    // closed is not brought back.
    let mut loaded = layout();
    loaded.open_float(float("detached", rect(0.0, 0.0, 100.0, 100.0)));
    loaded.open_float(float("palette", rect(0.0, 0.0, 100.0, 100.0)));
    loaded.close_float("palette");
    loaded.reconcile(&code);
    assert!(loaded.float("detached").is_some());
    assert!(loaded.float("palette").is_none());
    // Another root: the tree is the code's, the floats stay.
    let mut loaded = Layout::single("other");
    loaded.open_float(float("detached", rect(0.0, 0.0, 100.0, 100.0)));
    loaded.reconcile(&code);
    assert_eq!(loaded.root, code.root);
    assert!(loaded.float("detached").is_some());
}

#[test]
fn docked_repair_drops_floats_of_lost_docks_and_needs_no_zone_for_a_floats_dock() {
    let mut space = DockSpace::new();
    space.add_dock("centre", DockRules::default(), vec!["Editor".to_owned()]);
    space.add_dock("log", DockRules::default(), vec!["Log".to_owned()]);
    let mut layout = layout();
    layout.open_float(float("log", rect(0.0, 0.0, 100.0, 100.0)));
    layout.open_float(float("gone", rect(0.0, 0.0, 100.0, 100.0)));
    let mut docked = Docked::new(layout, space);
    let repair = docked.repair();
    assert_eq!(repair.floats_removed, ["gone"]);
    assert!(repair.added.is_empty());
    assert!(docked.layout.zone("log").is_none());
}

#[test]
fn a_tab_remembers_its_dock_and_neighbours() {
    let mut space = DockSpace::new();
    let surface = space.add_dock(
        "centre",
        DockRules::default(),
        vec!["A".to_owned(), "B".to_owned(), "C".to_owned()],
    );
    let id = |tab: &mut String| Id::unique(tab.as_str());
    let path = TabPath::new(surface, NodeIndex::root(), TabIndex(1));
    let origin = space.record_origin(path, id).unwrap().clone();
    assert_eq!(origin.tab, Id::unique("B"));
    assert_eq!(origin.dock, "centre");
    assert_eq!(origin.before, Some(Id::unique("A")));
    assert_eq!(origin.after, Some(Id::unique("C")));
    assert_eq!(space.origin(Id::unique("B")), Some(&origin));
    // A tab of no dock has no origin to record.
    let window = space.state_mut().add_window(vec!["W".to_owned()]);
    let path = TabPath::new(window, NodeIndex::root(), TabIndex(0));
    assert!(space.record_origin(path, id).is_none());
    assert_eq!(space.take_origin(Id::unique("B")), Some(origin));
    assert!(space.origins().is_empty());
}

#[cfg(feature = "serde")]
mod saved {
    use super::*;

    #[test]
    fn floats_and_closed_places_round_trip() {
        let mut layout = layout();
        let mut place = Place::on(&right(), rect(10.0, 20.0, 300.0, 200.0));
        place.maximized = true;
        layout
            .open_float(Float::dock("log", "log", FloatHost::Native, place))
            .minimized = true;
        layout.open_float(Float::dock(
            "tools",
            "tools",
            FloatHost::InWindow,
            Place::new(rect(5.0, 5.0, 100.0, 80.0)),
        ));
        layout.open_float(float("closed", rect(1.0, 2.0, 3.0, 4.0)));
        layout.close_float("closed");
        let json = serde_json::to_string(&layout).unwrap();
        assert!(json.contains("\"version\":2"));
        let loaded: Layout = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, layout);
    }

    #[test]
    fn a_version_1_layout_loads_with_no_float_and_unknown_fields_are_ignored() {
        let json = r#"{"version":1,"root":{"name":"centre","leaf":{"content":{"dock":"centre"}}},
            "floats":[{"key":"x","content":{"dock":"x"},"later":true}],"later":1}"#;
        let loaded: Layout = serde_json::from_str(json).unwrap();
        assert_eq!(loaded.root, Layout::single("centre").root);
        // A float missing its fields takes their defaults.
        assert_eq!(loaded.float("x").unwrap().place, Place::default());
    }

    #[test]
    fn a_layout_of_a_later_version_does_not_load() {
        let json = r#"{"version":99,"root":{"name":"centre","leaf":{"content":"empty"}}}"#;
        assert!(serde_json::from_str::<Layout>(json).is_err());
    }

    #[test]
    fn a_saved_0_27_window_becomes_a_native_float() {
        let json = include_str!("fixtures/docked-0.27-window.json");
        let mut loaded: Docked<String> = serde_json::from_str(json).unwrap();
        assert_eq!(
            loaded.space.dock_names().collect::<Vec<_>>(),
            ["centre", "window-1"]
        );
        let float = loaded.layout.float("window-1").unwrap();
        assert_eq!(float.host, FloatHost::Native);
        assert_eq!(float.content, Content::Dock("window-1".into()));
        assert_eq!(float.place.screen, None);
        assert_eq!(float.place.rect, rect(40.0, 60.0, 320.0, 240.0));
        assert!(float.minimized);
        // Its tabs moved with it, the active one kept.
        let tree = loaded.space.dock("window-1").unwrap();
        let tabs: Vec<&String> = tree.tabs().collect();
        assert_eq!(tabs, ["Log", "Find"]);
        // No legacy window is left, and the float's dock needs no zone.
        assert!(
            loaded
                .space
                .state()
                .iter_surfaces()
                .all(|surface| !matches!(surface, egui_stack::Surface::Window(..)))
        );
        assert!(loaded.repair().is_empty());
        // Saved again, it is a version 2 value and loads the same.
        let json = serde_json::to_string(&loaded).unwrap();
        let again: Docked<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(again.layout, loaded.layout);
        assert_eq!(
            again.space.dock_names().collect::<Vec<_>>(),
            ["centre", "window-1"]
        );
    }

    #[test]
    fn a_version_2_pair_keeps_its_legacy_windows() {
        // Only a pair saved before floats migrates.
        let json = include_str!("fixtures/docked-0.27-window.json").replacen(
            "\"version\": 1",
            "\"version\": 2",
            1,
        );
        let loaded: Docked<String> = serde_json::from_str(&json).unwrap();
        assert!(loaded.layout.floats.is_empty());
        assert_eq!(loaded.space.dock_names().collect::<Vec<_>>(), ["centre"]);
        assert!(
            loaded
                .space
                .state()
                .iter_surfaces()
                .any(|surface| matches!(surface, egui_stack::Surface::Window(..)))
        );
    }
}
