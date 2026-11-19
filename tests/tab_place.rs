//! A tab taken out of a dock space with its place (`DockSpace::take_with_place`) and put back
//! there (`DockSpace::restore_at`): the hide/show and close/reopen pattern, with no hidden tab
//! left in the tree.

use egui::{Id, Rect, pos2};
use egui_stack::{
    DockRules, DockSpace, NodeIndex, NodePath, TabDestination, TabIndex, TabInsert, TabPlace,
};

fn id(tab: &mut String) -> Id {
    Id::unique(tab.as_str())
}

fn tabs(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// The tabs of each group of the dock `dock`, in tree order.
fn groups(space: &DockSpace<String>, dock: &str) -> Vec<Vec<String>> {
    space
        .dock(dock)
        .unwrap()
        .iter()
        .filter_map(|node| node.get_leaf())
        .map(|leaf| leaf.tabs.clone())
        .collect()
}

/// A space with a "centre" dock of two groups, [A B C] | [D E], and a "side" dock with [S].
fn space() -> DockSpace<String> {
    let mut space = DockSpace::new();
    space.add_dock("centre", DockRules::default(), tabs(&["A", "B", "C"]));
    space
        .dock_mut("centre")
        .unwrap()
        .split_right(NodeIndex::root(), 0.5, tabs(&["D", "E"]));
    space.add_dock("side", DockRules::default(), tabs(&["S"]));
    space
}

#[test]
fn a_tab_taken_goes_back_to_the_same_group_and_index() {
    let mut space = space();
    let (tab, place) = space.take_with_place(Id::unique("B"), id).unwrap();
    assert_eq!(tab, "B");
    assert_eq!(place.tab(), Id::unique("B"));
    assert_eq!(place.dock(), "centre");
    // Gone from the tree: nothing hidden is left in it.
    assert_eq!(
        groups(&space, "centre"),
        [tabs(&["A", "C"]), tabs(&["D", "E"])]
    );
    assert!(space.state().find_tab(&"B".to_owned()).is_none());
    assert!(space.origins().is_empty());

    assert_eq!(space.restore_at(tab, place, id).unwrap(), "centre");
    assert_eq!(
        groups(&space, "centre"),
        [tabs(&["A", "B", "C"]), tabs(&["D", "E"])]
    );
    // It is its group's active tab again.
    let path = space.state().find_tab(&"B".to_owned()).unwrap();
    let leaf = space.state().leaf(path.node_path()).unwrap();
    assert_eq!(leaf.active, TabIndex(1));
}

#[test]
fn the_first_and_last_tabs_keep_their_ends() {
    let mut space = space();
    let (a, a_place) = space.take_with_place(Id::unique("A"), id).unwrap();
    let (e, e_place) = space.take_with_place(Id::unique("E"), id).unwrap();
    space.restore_at(e, e_place, id).unwrap();
    space.restore_at(a, a_place, id).unwrap();
    assert_eq!(
        groups(&space, "centre"),
        [tabs(&["A", "B", "C"]), tabs(&["D", "E"])]
    );
}

#[test]
fn a_tab_follows_its_neighbour_to_another_group() {
    let mut space = space();
    let (tab, place) = space.take_with_place(Id::unique("B"), id).unwrap();
    // The user moves A, the tab before B, to the other group: [C] | [D E A].
    let a = space.state().find_tab(&"A".to_owned()).unwrap();
    let d = space.state().find_tab(&"D".to_owned()).unwrap();
    space
        .state_mut()
        .move_tab(a, TabDestination::Node(d.node_path(), TabInsert::Append));
    assert_eq!(
        groups(&space, "centre"),
        [tabs(&["C"]), tabs(&["D", "E", "A"])]
    );
    assert_eq!(space.restore_at(tab, place, id).unwrap(), "centre");
    assert_eq!(
        groups(&space, "centre"),
        [tabs(&["C"]), tabs(&["D", "E", "A", "B"])]
    );
}

#[test]
fn a_tab_whose_group_is_gone_goes_to_the_focused_group_at_its_index() {
    let mut space = DockSpace::new();
    space.add_dock("centre", DockRules::default(), tabs(&["A", "B", "C"]));
    space
        .dock_mut("centre")
        .unwrap()
        .split_right(NodeIndex::root(), 0.5, tabs(&["D"]));
    // D was alone in its group: taking it takes the group.
    let (tab, place) = space.take_with_place(Id::unique("D"), id).unwrap();
    assert_eq!(groups(&space, "centre"), [tabs(&["A", "B", "C"])]);
    assert_eq!(space.restore_at(tab, place, id).unwrap(), "centre");
    // No neighbour to follow: the focused (here the only) group, at D's old index, 0.
    assert_eq!(groups(&space, "centre"), [tabs(&["D", "A", "B", "C"])]);
}

#[test]
fn a_tab_whose_neighbours_left_the_space_goes_to_the_focused_group() {
    let mut space = space();
    let (tab, place) = space.take_with_place(Id::unique("E"), id).unwrap();
    let d = space.state().find_tab(&"D".to_owned()).unwrap();
    space.state_mut().remove_tab(d);
    // [D E]'s group is gone; the focused group is the first, [A B C]; E's index was 1.
    assert_eq!(space.restore_at(tab, place, id).unwrap(), "centre");
    assert_eq!(groups(&space, "centre"), [tabs(&["A", "E", "B", "C"])]);
}

#[test]
fn a_tab_whose_dock_is_gone_goes_to_the_first_dock() {
    let mut space = space();
    let (tab, place) = space.take_with_place(Id::unique("S"), id).unwrap();
    // Taking the side dock's only tab leaves it empty, not removed.
    assert!(space.has_dock("side"));
    space.remove_dock("side");
    assert_eq!(space.restore_at(tab, place, id).unwrap(), "centre");
    // After the focused group's tabs: [D E], the group the split made.
    assert_eq!(
        groups(&space, "centre"),
        [tabs(&["A", "B", "C"]), tabs(&["D", "E", "S"])]
    );
}

#[test]
fn an_emptied_dock_takes_its_tab_back() {
    let mut space = space();
    let (tab, place) = space.take_with_place(Id::unique("S"), id).unwrap();
    assert!(space.dock("side").unwrap().is_empty());
    assert_eq!(space.restore_at(tab, place, id).unwrap(), "side");
    assert_eq!(groups(&space, "side"), [tabs(&["S"])]);
}

#[test]
fn a_space_with_no_dock_gives_the_tab_back() {
    let mut space = space();
    let (tab, place) = space.take_with_place(Id::unique("S"), id).unwrap();
    space.remove_dock("side");
    space.remove_dock("centre");
    assert_eq!(space.restore_at(tab, place, id).unwrap_err(), "S");
}

#[test]
fn an_unknown_tab_is_not_taken() {
    let mut space = space();
    assert!(space.take_with_place(Id::unique("Z"), id).is_none());
    // A tab of a legacy floating window is no dock's.
    let window = space.state_mut().add_window(tabs(&["W"]));
    assert!(space.take_with_place(Id::unique("W"), id).is_none());
    assert!(space.state().get_surface(window).is_some());
}

#[test]
fn tabs_go_back_across_docks() {
    let mut space = space();
    let (b, b_place) = space.take_with_place(Id::unique("B"), id).unwrap();
    let (s, s_place) = space.take_with_place(Id::unique("S"), id).unwrap();
    // Put back in the other order.
    assert_eq!(space.restore_at(b, b_place, id).unwrap(), "centre");
    assert_eq!(space.restore_at(s, s_place, id).unwrap(), "side");
    assert_eq!(
        groups(&space, "centre"),
        [tabs(&["A", "B", "C"]), tabs(&["D", "E"])]
    );
    assert_eq!(groups(&space, "side"), [tabs(&["S"])]);
}

/// Floats "B" out of the centre dock, then floats "C" into the same float beside it.
fn floated() -> (DockSpace<String>, String) {
    let mut space = space();
    let rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(200.0, 200.0));
    let b = space.state().find_tab(&"B".to_owned()).unwrap();
    let float = space.float_out(b, rect, id).unwrap();
    // C is now at index 1 of [A C].
    let c = space.state().find_tab(&"C".to_owned()).unwrap();
    space.record_origin(c, id).unwrap();
    let to = space.dock_surface(&float).unwrap();
    space.state_mut().move_tab(
        c,
        TabDestination::Node(NodePath::new(to, NodeIndex::root()), TabInsert::Append),
    );
    assert_eq!(groups(&space, &float), [tabs(&["B", "C"])]);
    (space, float)
}

#[test]
fn a_tab_taken_from_a_float_goes_back_into_it() {
    let (mut space, float) = floated();
    let (tab, place) = space.take_with_place(Id::unique("B"), id).unwrap();
    assert_eq!(place.dock(), float);
    // Its origin leaves with it.
    assert!(space.origin(Id::unique("B")).is_none());
    assert_eq!(space.restore_at(tab, place, id).unwrap(), float);
    assert_eq!(groups(&space, &float), [tabs(&["B", "C"])]);
    // Back in the float, it keeps where it came from, for the float's close.
    assert_eq!(space.origin(Id::unique("B")).unwrap().dock, "centre");
}

#[test]
fn a_tab_taken_from_a_float_since_closed_goes_where_it_came_from() {
    let (mut space, float) = floated();
    let (tab, place) = space.take_with_place(Id::unique("B"), id).unwrap();
    // The float closes, its other tab with it.
    space.remove_dock(&float);
    assert_eq!(space.restore_at(tab, place, id).unwrap(), "centre");
    // Beside A, the tab it was next to before it floated.
    assert_eq!(
        groups(&space, "centre"),
        [tabs(&["A", "B"]), tabs(&["D", "E"])]
    );
    assert!(space.origin(Id::unique("B")).is_none());
}

#[cfg(feature = "serde")]
mod saved {
    use super::*;
    use egui_kittest::Harness;
    use egui_stack::layout::{Content, Dir, Layout, Zone};
    use egui_stack::{DockArea, Docked, TabViewer};

    struct Viewer;

    impl TabViewer for Viewer {
        type Tab = String;

        fn id(&mut self, tab: &mut String) -> Id {
            id(tab)
        }

        fn title(&mut self, tab: &mut String) -> egui::WidgetText {
            tab.as_str().into()
        }

        fn ui(&mut self, ui: &mut egui::Ui, tab: &mut String) {
            ui.label(tab.as_str());
        }
    }

    #[test]
    fn a_place_survives_a_serde_round_trip() {
        let mut space = space();
        let (_, place) = space.take_with_place(Id::unique("B"), id).unwrap();
        let json = serde_json::to_string(&place).unwrap();
        let loaded: TabPlace = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, place);

        let (mut space, _) = floated();
        let (_, place) = space.take_with_place(Id::unique("B"), id).unwrap();
        let json = serde_json::to_string(&place).unwrap();
        let loaded: TabPlace = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, place);
    }

    #[test]
    fn a_place_outlives_a_layout_save_and_load() {
        let layout = Layout::new(Zone::split(
            "root",
            Dir::Horizontal,
            vec![
                Zone::leaf("side", Content::Dock("side".into())),
                Zone::leaf("centre", Content::Dock("centre".into())).with(|c| c.grow = true),
            ],
        ));
        let mut docked = Docked::new(layout, space());
        // Drawn once, so that every group has a rect to save.
        {
            let mut harness = Harness::new_ui(|ui| {
                ui.columns(2, |columns| {
                    DockArea::dock(&mut docked.space, "side")
                        .show_inside(&mut columns[0], &mut Viewer);
                    DockArea::dock(&mut docked.space, "centre")
                        .show_inside(&mut columns[1], &mut Viewer);
                });
            });
            harness.run();
        }
        let (tab, place) = docked.space.take_with_place(Id::unique("B"), id).unwrap();
        let saved = (
            serde_json::to_string(&docked).unwrap(),
            serde_json::to_string(&(&tab, &place)).unwrap(),
        );

        let mut loaded: Docked<String> = serde_json::from_str(&saved.0).unwrap();
        let (tab, place): (String, TabPlace) = serde_json::from_str(&saved.1).unwrap();
        assert_eq!(loaded.space.restore_at(tab, place, id).unwrap(), "centre");
        assert_eq!(
            groups(&loaded.space, "centre"),
            [tabs(&["A", "B", "C"]), tabs(&["D", "E"])]
        );
    }
}
