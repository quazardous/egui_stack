//! A layout and its dock space kept as one value, repaired when they
//! disagree.

use egui_stack::layout::{Content, Dir, Layout, Zone, ZoneName};
use egui_stack::{DockRules, DockSpace, Docked};

fn docked() -> Docked<String> {
    let mut space = DockSpace::new();
    space.add_dock("left", DockRules::default(), vec!["Files".to_owned()]);
    space.add_dock("centre", DockRules::default(), vec!["Editor".to_owned()]);
    let layout = Layout::new(Zone::split(
        "root",
        Dir::Horizontal,
        vec![
            Zone::leaf("left", Content::Dock("left".into())),
            Zone::leaf("centre", Content::Dock("centre".into())).with(|c| c.grow = true),
        ],
    ));
    Docked::new(layout, space)
}

#[test]
fn a_pair_that_agrees_needs_no_repair() {
    let mut docked = docked();
    assert!(docked.repair().is_empty());
}

#[test]
fn a_dock_removed_from_the_space_takes_its_zone_with_it() {
    let mut docked = docked();
    docked.space.remove_dock("left");
    let repair = docked.repair();
    assert_eq!(repair.removed, [ZoneName::from("left")]);
    assert!(docked.layout.zone("left").is_none());
    // Again: nothing left to repair.
    assert!(docked.repair().is_empty());
}

#[test]
fn a_dock_no_zone_shows_gets_a_hidden_zone() {
    let mut docked = docked();
    docked
        .space
        .add_dock("log", DockRules::default(), vec!["Log".to_owned()]);
    let repair = docked.repair();
    assert_eq!(repair.added, [ZoneName::from("log")]);
    let zone = docked.layout.zone("log").unwrap();
    assert!(zone.state.hidden);
}

#[cfg(feature = "serde")]
#[test]
fn the_pair_saves_and_loads_as_one_value() {
    let mut saved = docked();
    saved.layout.zone_mut("left").unwrap().state.size = Some(180.0);
    let json = serde_json::to_string(&saved).unwrap();
    let loaded: Docked<String> = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.layout, saved.layout);
    assert_eq!(
        loaded.space.dock_names().collect::<Vec<_>>(),
        ["left", "centre"]
    );
}
