//! The built-in tab actions are public `DockState` methods, and `context_menu` gets the `TabPath`.

use egui::{Id, Pos2, Ui, WidgetText, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable as _};
use egui_stack::{
    DockArea, DockState, NodeIndex, SurfaceIndex, TabIndex, TabPath, TabViewer,
    tab_viewer::OnCloseResponse,
};

struct Viewer {
    answer: OnCloseResponse,
    closeable: bool,
    menu_paths: Vec<TabPath>,
}

impl Viewer {
    fn new(answer: OnCloseResponse) -> Self {
        Self {
            answer,
            closeable: true,
            menu_paths: Vec::new(),
        }
    }
}

impl TabViewer for Viewer {
    type Tab = String;

    fn id(&mut self, tab: &mut String) -> Id {
        Id::unique(tab.as_str())
    }

    fn title(&mut self, tab: &mut String) -> WidgetText {
        tab.as_str().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut String) {
        ui.label(format!("Body of {tab}"));
    }

    fn context_menu(&mut self, ui: &mut Ui, _tab: &mut String, path: TabPath) {
        self.menu_paths.push(path);
        let _ = ui.button("Rename");
    }

    fn is_closeable(&self, _tab: &String) -> bool {
        self.closeable
    }

    fn on_close(&mut self, _tab: &mut String) -> OnCloseResponse {
        self.answer
    }
}

fn dock() -> DockState<String> {
    let mut dock_state = DockState::new(vec!["First".to_owned(), "Second".to_owned()]);
    dock_state
        .main_surface_mut()
        .split_right(NodeIndex::root(), 0.5, vec!["Third".to_owned()]);
    dock_state
}

/// `First` and `Second` sit in the root's left child, `Third` in its right one.
fn second() -> TabPath {
    TabPath::new(SurfaceIndex::main(), NodeIndex(1), TabIndex(1))
}

fn titles(dock_state: &DockState<String>) -> Vec<&str> {
    dock_state
        .iter_all_tabs()
        .map(|(_, tab)| tab.as_str())
        .collect()
}

#[test]
fn close_tab_removes_on_close() {
    let mut dock_state = dock();
    let answer = dock_state
        .close_tab(second(), &mut Viewer::new(OnCloseResponse::Close))
        .unwrap();
    assert_eq!(answer, OnCloseResponse::Close);
    assert_eq!(titles(&dock_state), ["First", "Third"]);
}

#[test]
fn close_tab_focuses_on_focus() {
    let mut dock_state = dock();
    let answer = dock_state
        .close_tab(second(), &mut Viewer::new(OnCloseResponse::Focus))
        .unwrap();
    assert_eq!(answer, OnCloseResponse::Focus);
    assert_eq!(titles(&dock_state), ["First", "Second", "Third"]);
    let leaf = second().node_path();
    assert_eq!(dock_state.focused_leaf(), Some(leaf));
    assert_eq!(dock_state.leaf(leaf).unwrap().active, TabIndex(1));
}

#[test]
fn close_tab_leaves_a_tab_that_is_not_closeable() {
    let mut dock_state = dock();
    let mut viewer = Viewer::new(OnCloseResponse::Close);
    viewer.closeable = false;
    let answer = dock_state.close_tab(second(), &mut viewer).unwrap();
    assert_eq!(answer, OnCloseResponse::Ignore);
    assert_eq!(titles(&dock_state), ["First", "Second", "Third"]);
}

#[test]
fn close_tab_refuses_a_missing_tab() {
    let mut dock_state = dock();
    let missing = TabPath::new(SurfaceIndex::main(), NodeIndex(1), TabIndex(7));
    assert!(
        dock_state
            .close_tab(missing, &mut Viewer::new(OnCloseResponse::Close))
            .is_err()
    );
}

#[test]
fn focus_tab_activates_and_focuses() {
    let mut dock_state = dock();
    dock_state.focus_tab(second()).unwrap();
    let leaf = second().node_path();
    assert_eq!(dock_state.focused_leaf(), Some(leaf));
    assert_eq!(dock_state.leaf(leaf).unwrap().active, TabIndex(1));
}

#[test]
fn eject_tab_opens_a_window() {
    let mut dock_state = dock();
    let window = dock_state.eject_tab(second(), Pos2::new(40.0, 50.0));
    assert!(!window.is_main());
    assert_eq!(dock_state[window].num_tabs(), 1);
    assert_eq!(titles(&dock_state), ["First", "Third", "Second"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn context_menu_gets_the_tab_path() {
    let mut harness = Harness::builder().with_size([600.0, 300.0]).build_ui_state(
        |ui, (dock_state, viewer): &mut (DockState<String>, Viewer)| {
            DockArea::new(dock_state).show_inside(ui, viewer);
        },
        (dock(), Viewer::new(OnCloseResponse::Close)),
    );
    harness.run();
    harness
        .get_by_role_and_label(Role::Tab, "Second")
        .click_secondary();
    harness.run();
    assert_eq!(harness.state().1.menu_paths.last(), Some(&second()));
}
