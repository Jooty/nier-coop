use eframe::egui::{
    Align, Color32, CornerRadius, Label, Layout, Rect, Response, RichText, Sense, Stroke,
    StrokeKind, Ui, pos2, vec2,
};

use crate::app::pages::marks::{game_mark, house_icon_width, paint_house_icon, status_dot};
use crate::app::pages::tone::tone_color;
use crate::app::palette::{BAND, HAIRLINE, INK, INK_MUTED};
use crate::app::style::{body_font, garamond_family};
use crate::app::{GameLook, Launcher, Tab};
use crate::home::card_view;

pub(super) const TOP_BAR_HEIGHT_PX: f32 = 56.0;
const TOP_BAR_GAP_PX: f32 = 18.0;
const HOME_BUTTON_HEIGHT_PX: f32 = 32.0;
const HOME_BUTTON_LEFT_PADDING_PX: f32 = 10.0;
const HOME_BUTTON_RIGHT_PADDING_PX: f32 = 12.0;
const HOME_BUTTON_ICON_GAP_PX: f32 = 8.0;
const HOME_BUTTON_CORNER_RADIUS_PX: u8 = 6;
const GAME_MARK_TO_TITLE_GAP_PX: f32 = 10.0;
const TABS_EXTRA_LEFT_MARGIN_PX: f32 = 14.0;
const TAB_SIDE_PADDING_PX: f32 = 16.0;
const TAB_UNDERLINE_HEIGHT_PX: f32 = 2.0;
const MOD_CHIP_DOT_GAP_PX: f32 = 8.0;

pub(super) fn draw_top_bar(ui: &mut Ui, tab: &mut Tab, launcher: &mut Launcher, look: GameLook) {
    // The bottom border goes down first so the selected tab's underline can cover it.
    ui.painter().hline(
        ui.ctx().content_rect().x_range(),
        ui.max_rect().bottom() - 0.5,
        Stroke::new(1.0, HAIRLINE),
    );
    ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        if home_button(ui).clicked() {
            launcher.go_home();
        }
        ui.add_space(TOP_BAR_GAP_PX);
        game_mark(ui, look);
        ui.add_space(GAME_MARK_TO_TITLE_GAP_PX);
        ui.add(
            Label::new(
                RichText::new(launcher.profile.display_name)
                    .font(eframe::egui::FontId::new(24.0, garamond_family())),
            )
            .selectable(false)
            .extend(),
        );
        ui.add_space(TOP_BAR_GAP_PX + TABS_EXTRA_LEFT_MARGIN_PX);
        for (value, label) in Tab::ALL {
            if tab_button(ui, label, *tab == value, crate::app::palette::INK).clicked() {
                *tab = value;
            }
        }
        ui.add_space(TOP_BAR_GAP_PX);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            mod_chip(ui, launcher);
        });
    });
}

fn home_button(ui: &mut Ui) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap("Home".to_owned(), body_font(), INK_MUTED);
    let width = HOME_BUTTON_LEFT_PADDING_PX
        + house_icon_width()
        + HOME_BUTTON_ICON_GAP_PX
        + galley.size().x
        + HOME_BUTTON_RIGHT_PADDING_PX;
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, HOME_BUTTON_HEIGHT_PX), Sense::click());
    let (fill, color) = if response.hovered() {
        (BAND, INK)
    } else {
        (Color32::TRANSPARENT, INK_MUTED)
    };
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(HOME_BUTTON_CORNER_RADIUS_PX),
        fill,
        Stroke::new(1.0, HAIRLINE),
        StrokeKind::Inside,
    );
    let icon_left = rect.left() + HOME_BUTTON_LEFT_PADDING_PX;
    paint_house_icon(painter, pos2(icon_left, rect.center().y), color);
    let text_left = icon_left + house_icon_width() + HOME_BUTTON_ICON_GAP_PX;
    painter.galley(
        pos2(text_left, rect.center().y - galley.size().y / 2.0),
        galley,
        color,
    );
    response
}

fn tab_button(ui: &mut Ui, label: &str, selected: bool, accent: Color32) -> Response {
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), body_font(), INK_MUTED);
    let size = vec2(
        galley.size().x + 2.0 * TAB_SIDE_PADDING_PX,
        ui.available_height().max(TOP_BAR_HEIGHT_PX),
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let color = if selected || response.hovered() {
        INK
    } else {
        INK_MUTED
    };
    let painter = ui.painter();
    painter.galley(rect.center() - galley.size() / 2.0, galley, color);
    if selected {
        let underline = Rect::from_min_max(
            pos2(rect.left(), rect.bottom() - TAB_UNDERLINE_HEIGHT_PX),
            rect.right_bottom(),
        );
        painter.rect_filled(underline, CornerRadius::ZERO, accent);
    }
    response
}

fn mod_chip(ui: &mut Ui, launcher: &Launcher) {
    let Some(card) = launcher
        .cards
        .iter()
        .find(|card| card.profile.game_id == launcher.profile.game_id)
    else {
        return;
    };
    let view = card_view(card);
    ui.add(
        Label::new(RichText::new(view.mod_text).color(INK_MUTED))
            .selectable(false)
            .truncate(),
    );
    ui.add_space(MOD_CHIP_DOT_GAP_PX);
    status_dot(ui, tone_color(view.mod_tone));
}
