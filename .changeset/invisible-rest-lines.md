---
egui_stack: major
---

The default look draws no line between two leaves at rest, where it drew three (the left body's border, the separator and the right body's border): `TabBodyStyle::stroke` is now `Stroke::NONE` in `TabBodyStyle::from_egui` (it already was in `Default`), and `SeparatorStyle::color_idle` is transparent in `SeparatorStyle::from_egui` and `Default`. The separator still shows on hover and drag in `color_hovered` and `color_dragged`, and its width and grab area are unchanged. Migration: for the former look, set them after building the style; from an `egui::Style` `s`, `style.tab.tab_body.stroke = s.visuals.widgets.noninteractive.bg_stroke;` and `style.separator.color_idle = s.visuals.widgets.noninteractive.bg_stroke.color;` (`Style::default()` had `color_idle: Color32::BLACK`).
