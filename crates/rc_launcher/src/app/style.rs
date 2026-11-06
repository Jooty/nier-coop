use eframe::egui::{Color32, CornerRadius, Stroke, Visuals};

const WINDOW_BACKGROUND: Color32 = Color32::from_rgb(15, 15, 15);
const FRAME_BACKGROUND: Color32 = Color32::from_rgb(41, 74, 122);
const FRAME_HOVERED: Color32 = Color32::from_rgb(66, 150, 250);
const BUTTON: Color32 = Color32::from_rgb(40, 96, 160);
const BUTTON_HOVERED: Color32 = Color32::from_rgb(66, 150, 250);
const BUTTON_ACTIVE: Color32 = Color32::from_rgb(15, 135, 250);
const BORDER: Color32 = Color32::from_rgb(70, 70, 82);
const TEXT: Color32 = Color32::from_rgb(235, 235, 235);

pub fn imgui_dark_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = WINDOW_BACKGROUND;
    visuals.window_fill = WINDOW_BACKGROUND;
    visuals.extreme_bg_color = Color32::from_rgb(10, 10, 10);
    visuals.faint_bg_color = Color32::from_rgb(24, 24, 24);
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = BUTTON;
    visuals.selection.stroke = Stroke::new(1.0, TEXT);
    visuals.window_corner_radius = CornerRadius::ZERO;
    visuals.menu_corner_radius = CornerRadius::ZERO;
    let states = [
        (&mut visuals.widgets.noninteractive, WINDOW_BACKGROUND),
        (&mut visuals.widgets.inactive, BUTTON),
        (&mut visuals.widgets.hovered, BUTTON_HOVERED),
        (&mut visuals.widgets.active, BUTTON_ACTIVE),
        (&mut visuals.widgets.open, FRAME_BACKGROUND),
    ];
    for (widget, fill) in states {
        widget.corner_radius = CornerRadius::ZERO;
        widget.weak_bg_fill = fill;
        widget.bg_fill = fill;
        widget.bg_stroke = Stroke::new(1.0, BORDER);
        widget.fg_stroke = Stroke::new(1.0, TEXT);
    }
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, FRAME_HOVERED);
    visuals
}
