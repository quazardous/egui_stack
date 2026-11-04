//! A toolbar row: what fits and what goes into the More menu, what a click says, what screen
//! readers read, and the keyboard along it.

use egui::accesskit::{Role, Toggled};
use egui::{Key, Vec2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_stack::TabFocus;
use egui_stack::toolbar::{
    self, MORE, Orientation, Tool, ToolDisplay, ToolKind, ToolRow, laid_out,
};

const NAME: &str = "test.row";

/// What the row is shown with, and what it said.
struct State {
    display: ToolDisplay,
    orientation: Orientation,
    tab_focus: TabFocus,
    bold: bool,
    /// Save waits: a spinner, which repaints on every pass.
    busy: bool,
    clicked: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        State {
            display: ToolDisplay::Icons,
            orientation: Orientation::Horizontal,
            tab_focus: TabFocus::Roving,
            bold: false,
            busy: false,
            clicked: Vec::new(),
        }
    }
}

/// A square icon painted as a filled box.
fn icon(painter: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    painter.rect_filled(rect.shrink(2.0), 0.0, color);
}

fn row(ui: &mut egui::Ui, state: &mut State) {
    // A button before the row and one after, so that Tab has somewhere to come from and go.
    let _ = ui.button("Before");
    let out = ToolRow::new(NAME)
        .title("Formatting")
        .display(state.display)
        .orientation(state.orientation)
        .tab_focus(state.tab_focus)
        .show(
            ui,
            [
                Tool::button("cut", "Cut").icon_painter(icon),
                Tool::button("copy", "Copy")
                    .icon_painter(icon)
                    .enabled(false),
                Tool::button("paste", "Paste").icon_painter(icon),
                Tool::separator(),
                Tool::toggle("bold", "Bold", state.bold).icon_painter(icon),
                Tool::radio("left", "Left", true).icon_painter(icon),
                Tool::button("save", "Save")
                    .icon_painter(icon)
                    .busy(state.busy)
                    .tooltip("Save (Ctrl+S)"),
            ],
        );
    if let Some(id) = out.clicked {
        if id == "bold" {
            state.bold = !state.bold;
        }
        state.clicked.push(id);
    }
    let _ = ui.button("After");
}

fn harness(width: f32, state: State) -> Harness<'static, State> {
    Harness::builder()
        .with_size(vec2(width, 300.0))
        .build_ui_state(row, state)
}

fn shown(h: &Harness<'_, State>) -> Vec<String> {
    laid_out(&h.ctx, NAME)
        .unwrap()
        .tools
        .iter()
        .filter(|t| !t.overflow)
        .map(|t| t.id.clone())
        .collect()
}

fn overflowed(h: &Harness<'_, State>) -> Vec<String> {
    laid_out(&h.ctx, NAME)
        .unwrap()
        .tools
        .iter()
        .filter(|t| t.overflow)
        .map(|t| t.id.clone())
        .collect()
}

fn focused_name(h: &Harness<'_, State>) -> Option<String> {
    h.root()
        .children_recursive()
        .find(|node| node.is_focused())
        .and_then(|node| node.accesskit_node().label().map(|l| l.to_string()))
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn every_tool_shows_when_there_is_room() {
    let mut h = harness(600.0, State::default());
    h.run();
    assert_eq!(
        shown(&h),
        ["cut", "copy", "paste", "", "bold", "left", "save"]
    );
    let laid = laid_out(&h.ctx, NAME).unwrap();
    assert_eq!(laid.overflow, 0);
    assert_eq!(laid.more, None);
    // Icons alone: square tools, one after the other.
    let rects: Vec<_> = laid.tools.iter().map(|t| t.rect.unwrap()).collect();
    assert!((rects[0].width() - rects[0].height()).abs() < 0.5);
    assert!(rects.windows(2).all(|w| w[0].right() < w[1].left()));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn tools_that_do_not_fit_go_into_more_in_their_order() {
    let mut h = harness(600.0, State::default());
    h.run();
    let laid = laid_out(&h.ctx, NAME).unwrap();
    let cut = laid.tools[4].rect.unwrap().right() - laid.rect.left();
    // Room for the first five and the More button, not for the last two.
    let panel_margin = laid.rect.left() - h.ctx.content_rect().left();
    h.set_size(vec2(cut + 30.0 + 2.0 * panel_margin, 300.0));
    h.run();
    assert_eq!(shown(&h), ["cut", "copy", "paste", "", "bold"]);
    assert_eq!(overflowed(&h), ["left", "save"]);
    let laid = laid_out(&h.ctx, NAME).unwrap();
    assert_eq!(laid.overflow, 2);
    assert!(laid.more.is_some());
    // Narrower, past the separator: it ends neither the row nor starts the menu.
    let cut = laid.tools[2].rect.unwrap().right() - laid.rect.left();
    h.set_size(vec2(cut + 30.0 + 2.0 * panel_margin, 300.0));
    h.run();
    assert_eq!(shown(&h), ["cut", "copy", "paste"]);
    assert_eq!(overflowed(&h), ["bold", "left", "save"]);
    // The More button is named after the row, and opens the rest as a menu.
    let more = h.get_by_role_and_label(Role::Button, "More tools: Formatting");
    assert_eq!(more.accesskit_node().data().is_expanded(), Some(false));
    more.click();
    h.run();
    let more = h.get_by_role_and_label(Role::Button, "More tools: Formatting");
    assert_eq!(more.accesskit_node().data().is_expanded(), Some(true));
    h.get_by_role_and_label(Role::Button, "Bold").click();
    h.run();
    assert_eq!(h.state().clicked, ["bold"]);
    assert!(h.state().bold);
    // The menu closed with the click.
    h.run();
    assert!(h.query_by_role_and_label(Role::Button, "Bold").is_none());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_click_says_which_tool_and_a_disabled_one_says_nothing() {
    let mut h = harness(600.0, State::default());
    h.run();
    h.get_by_role_and_label(Role::Button, "Cut").click();
    h.run();
    h.get_by_role_and_label(Role::Button, "Copy").click();
    h.run();
    h.get_by_role_and_label(Role::Button, "Bold").click();
    h.run();
    h.run();
    assert_eq!(h.state().clicked, ["cut", "bold"]);
    let bold = h.get_by_role_and_label(Role::Button, "Bold");
    assert_eq!(bold.accesskit_node().data().toggled(), Some(Toggled::True));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn screen_readers_read_a_named_toolbar_and_each_tool_state() {
    let state = State {
        busy: true,
        ..State::default()
    };
    let mut h = harness(600.0, state);
    // A spinner repaints on every pass: step rather than run.
    h.run_steps(3);
    let bar = h.get_by_role_and_label(Role::Toolbar, "Formatting");
    let names: Vec<String> = bar
        .children_recursive()
        .filter_map(|n| n.accesskit_node().label().map(|l| l.to_string()))
        .collect();
    for name in ["Cut", "Copy", "Paste", "Bold", "Left", "Save"] {
        assert!(names.iter().any(|n| n == name), "{name} in {names:?}");
    }
    let node = |role, name| {
        h.get_by_role_and_label(role, name)
            .accesskit_node()
            .data()
            .clone()
    };
    assert!(node(Role::Button, "Copy").is_disabled());
    assert_eq!(node(Role::Button, "Bold").toggled(), Some(Toggled::False));
    assert_eq!(
        node(Role::RadioButton, "Left").toggled(),
        Some(Toggled::True)
    );
    assert!(node(Role::Button, "Save").is_busy());
    assert_eq!(node(Role::Button, "Cut").toggled(), None);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_row_is_one_tab_stop_with_the_arrows_along_it() {
    let mut h = harness(600.0, State::default());
    h.run();
    // Tab from the button before lands on the first tool; Tab again leaves the row.
    h.get_by_role_and_label(Role::Button, "Before").focus();
    h.run();
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(focused_name(&h).as_deref(), Some("Cut"));
    let press = |h: &mut Harness<'_, State>, key| {
        h.key_press(key);
        h.run();
        focused_name(h)
    };
    // Copy is disabled: passed over.
    assert_eq!(press(&mut h, Key::ArrowRight).as_deref(), Some("Paste"));
    assert_eq!(press(&mut h, Key::ArrowRight).as_deref(), Some("Bold"));
    assert_eq!(press(&mut h, Key::End).as_deref(), Some("Save"));
    assert_eq!(press(&mut h, Key::ArrowRight).as_deref(), Some("Cut"));
    assert_eq!(press(&mut h, Key::ArrowLeft).as_deref(), Some("Save"));
    assert_eq!(press(&mut h, Key::Home).as_deref(), Some("Cut"));
    assert_eq!(press(&mut h, Key::ArrowRight).as_deref(), Some("Paste"));
    assert_eq!(
        toolbar::focused(&h.ctx, NAME).as_deref(),
        Some("paste"),
        "the focus API reads it"
    );
    // Out with Tab, back with Shift+Tab: on the tool focused last.
    assert_eq!(press(&mut h, Key::Tab).as_deref(), Some("After"));
    h.key_press_modifiers(egui::Modifiers::SHIFT, Key::Tab);
    h.run();
    assert_eq!(focused_name(&h).as_deref(), Some("Paste"));
    // Enter clicks.
    h.key_press(Key::Enter);
    h.run();
    assert_eq!(h.state().clicked, ["paste"]);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_focus_api_moves_the_tab_stop() {
    let mut h = harness(600.0, State::default());
    h.run();
    assert!(toolbar::focus(&h.ctx, NAME, "bold"));
    assert!(!toolbar::focus(&h.ctx, NAME, "copy"), "disabled");
    assert!(!toolbar::focus(&h.ctx, NAME, MORE), "no More button");
    assert!(toolbar::focus(&h.ctx, NAME, "bold"));
    h.run();
    assert_eq!(focused_name(&h).as_deref(), Some("Bold"));
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(focused_name(&h).as_deref(), Some("Left"));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn the_more_button_is_in_the_arrows_round() {
    let mut h = harness(600.0, State::default());
    h.run();
    let laid = laid_out(&h.ctx, NAME).unwrap();
    let cut = laid.tools[2].rect.unwrap().right() - laid.rect.left();
    let panel_margin = laid.rect.left() - h.ctx.content_rect().left();
    h.set_size(vec2(cut + 30.0 + 2.0 * panel_margin, 300.0));
    h.run();
    assert!(toolbar::focus(&h.ctx, NAME, "paste"));
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(focused_name(&h).as_deref(), Some("More tools: Formatting"));
    assert_eq!(toolbar::focused(&h.ctx, NAME).as_deref(), Some(MORE));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn with_each_tool_a_tab_stop_the_arrows_are_egui_s() {
    let state = State {
        tab_focus: TabFocus::Each,
        ..State::default()
    };
    let mut h = harness(600.0, state);
    h.run();
    h.get_by_role_and_label(Role::Button, "Before").focus();
    h.run();
    let mut stops = Vec::new();
    for _ in 0..7 {
        h.key_press(Key::Tab);
        h.run();
        stops.extend(focused_name(&h));
    }
    assert_eq!(
        stops,
        ["Cut", "Paste", "Bold", "Left", "Save", "After", "Before"]
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn labels_show_as_the_display_says() {
    let widths = |display| {
        let state = State {
            display,
            ..State::default()
        };
        let mut h = harness(800.0, state);
        h.run();
        laid_out(&h.ctx, NAME).unwrap().tools[0]
            .rect
            .unwrap()
            .width()
    };
    let (icons, beside, text) = (
        widths(ToolDisplay::Icons),
        widths(ToolDisplay::Beside),
        widths(ToolDisplay::Text),
    );
    assert!(icons < text && text < beside, "{icons} {text} {beside}");
    // With its label shown, a tool has no tooltip of its label.
    let state = State {
        display: ToolDisplay::Text,
        ..State::default()
    };
    let mut h = harness(800.0, state);
    h.run();
    assert_eq!(laid_out(&h.ctx, NAME).unwrap().display, ToolDisplay::Text);
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_column_runs_down_and_its_arrows_are_up_and_down() {
    let state = State {
        orientation: Orientation::Vertical,
        ..State::default()
    };
    let mut h = harness(300.0, state);
    h.run();
    let laid = laid_out(&h.ctx, NAME).unwrap();
    let rects: Vec<_> = laid.tools.iter().map(|t| t.rect.unwrap()).collect();
    assert!(rects.windows(2).all(|w| w[0].bottom() < w[1].top()));
    assert!(toolbar::focus(&h.ctx, NAME, "cut"));
    h.run();
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(focused_name(&h).as_deref(), Some("Cut"), "not along it");
    h.key_press(Key::ArrowDown);
    h.run();
    assert_eq!(focused_name(&h).as_deref(), Some("Paste"));
    let bar = h.get_by_role_and_label(Role::Toolbar, "Formatting");
    assert_eq!(
        bar.accesskit_node().data().orientation(),
        Some(egui::accesskit::Orientation::Vertical)
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_spacer_pushes_the_tools_after_it_to_the_end() {
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 100.0))
        .build_ui(|ui| {
            ToolRow::new("spaced").display(ToolDisplay::Text).show(
                ui,
                [
                    Tool::button("a", "A"),
                    Tool::spacer(),
                    Tool::button("b", "B"),
                    Tool::custom("zoom", "Zoom", Vec2::new(60.0, 20.0), |ui| {
                        ui.label("100%");
                    }),
                ],
            );
        });
    h.run();
    let laid = laid_out(&h.ctx, "spaced").unwrap();
    let ends: Vec<_> = laid.tools.iter().map(|t| t.rect.unwrap()).collect();
    assert!((ends[3].right() - laid.rect.right()).abs() < 0.5);
    assert!(laid.rect.width() > 300.0, "{:?}", laid.rect);
    assert!(ends[2].left() - ends[0].right() > 100.0, "{ends:?}");
    assert_eq!(laid.tools[3].kind, ToolKind::Custom);
    assert!(h.query_by_label("100%").is_some());
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn a_menu_tool_opens_the_caller_s_menu() {
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .build_ui_state(
            |ui, picked: &mut Vec<&'static str>| {
                ToolRow::new("menus").show(
                    ui,
                    [Tool::menu("view", "View", |ui| {
                        if ui.button("Zoom in").clicked() {
                            picked.push("zoom in");
                        }
                    })],
                );
            },
            Vec::new(),
        );
    h.run();
    let view = h.get_by_role_and_label(Role::Button, "View");
    assert_eq!(view.accesskit_node().data().is_expanded(), Some(false));
    assert_eq!(
        view.accesskit_node().data().has_popup(),
        Some(egui::accesskit::HasPopup::Menu)
    );
    view.click();
    h.run();
    let view = h.get_by_role_and_label(Role::Button, "View");
    assert_eq!(view.accesskit_node().data().is_expanded(), Some(true));
    h.get_by_role_and_label(Role::Button, "Zoom in").click();
    h.run();
    assert_eq!(h.state(), &["zoom in"]);
    assert_eq!(
        laid_out(&h.ctx, "menus").unwrap().tools[0].kind,
        ToolKind::Menu
    );
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn enter_opens_the_more_menu() {
    let mut h = harness(600.0, State::default());
    h.run();
    let laid = laid_out(&h.ctx, NAME).unwrap();
    let cut = laid.tools[2].rect.unwrap().right() - laid.rect.left();
    let panel_margin = laid.rect.left() - h.ctx.content_rect().left();
    h.set_size(vec2(cut + 30.0 + 2.0 * panel_margin, 300.0));
    h.run();
    assert!(toolbar::focus(&h.ctx, NAME, MORE));
    h.run();
    assert_eq!(toolbar::focused(&h.ctx, NAME).as_deref(), Some(MORE));
    h.key_press(Key::Enter);
    h.run();
    let more = h.get_by_role_and_label(Role::Button, "More tools: Formatting");
    assert_eq!(more.accesskit_node().data().is_expanded(), Some(true));
}

#[test]
#[cfg_attr(miri, ignore = "drives a whole UI; too slow under Miri")]
fn assistive_technology_focuses_any_tool_and_it_becomes_the_stop() {
    let mut h = harness(600.0, State::default());
    h.run();
    h.get_by_role_and_label(Role::Button, "Bold").focus();
    h.run();
    assert_eq!(focused_name(&h).as_deref(), Some("Bold"));
    h.key_press(Key::ArrowRight);
    h.run();
    assert_eq!(focused_name(&h).as_deref(), Some("Left"));
}
