use eframe::egui::{Align2, FontId, Id, Rect, Response, Sense, Stroke, Ui, UiBuilder, pos2, vec2};

use crate::app::pages::game_title::game_title_job;
use crate::app::palette::{
    HAIRLINE_STRONG, HARD_SHADOW, HOVER_WASH, INK, INK_MUTED, INK_ON_SAND, SAND,
};
use crate::app::parts::{caps_galley, paint_triangle_left, spaced_job};
use crate::app::style::light_family;
use crate::app::{Launcher, Tab, WINDOW_TITLE, game_look};

pub(super) const TOP_BAR_HEIGHT_PX: f32 = 60.0;
const BACK_TRIANGLE_WIDTH_PX: f32 = 7.0;
const BACK_TRIANGLE_TO_TEXT_PX: f32 = 9.0;
const BACK_RIGHT_PADDING_PX: f32 = 22.0;
const BACK_DIVIDER_HEIGHT_PX: f32 = 28.0;
const TITLE_LEFT_GAP_PX: f32 = 22.0;
const TITLE_COLUMN_WIDTH_PX: f32 = 300.0;
const TITLE_TO_TABS_MIN_GAP_PX: f32 = 24.0;
const TOP_BAR_GARAMOND_TITLE_PX: f32 = 28.0;
const TOP_BAR_CAPS_TITLE_PX: f32 = 22.0;
const TAB_HEIGHT_PX: f32 = 32.0;
const TAB_GAP_PX: f32 = 6.0;
const TAB_SIDE_PADDING_PX: f32 = 18.0;
const TAB_SHADOW_OFFSET_PX: f32 = 3.0;
const HOME_TITLE_SIZE_PX: f32 = 22.0;
const HOME_TITLE_LETTER_SPACING_EM: f32 = 0.26;
const LAUNCHER_VERSION_PREFIX: &str = "Launcher ";

pub(super) fn draw_home_bar(ui: &mut Ui) {
    let rect = ui.max_rect();
    let middle = rect.center().y;
    let painter = ui.painter();
    let title = painter.layout_job(spaced_job(
        &WINDOW_TITLE.to_uppercase(),
        FontId::new(HOME_TITLE_SIZE_PX, light_family()),
        INK,
        HOME_TITLE_LETTER_SPACING_EM,
    ));
    painter.galley(pos2(rect.left(), middle - title.size().y / 2.0), title, INK);
    let version = caps_galley(
        painter,
        &format!("{LAUNCHER_VERSION_PREFIX}{}", env!("CARGO_PKG_VERSION")),
        INK_MUTED,
    );
    let at = Align2::RIGHT_CENTER
        .anchor_size(pos2(rect.right(), middle), version.size())
        .min;
    painter.galley(at, version, INK_MUTED);
}

pub(super) fn draw_top_bar(ui: &mut Ui, tab: &mut Tab, launcher: &mut Launcher) {
    let rect = ui.max_rect();
    let middle = rect.center().y;
    let back = back_button(ui, rect.left(), middle);
    if back.clicked() {
        launcher.go_home();
    }
    let tabs_width = tabs_width(ui);
    let title_left = back.rect.right() + TITLE_LEFT_GAP_PX;
    let face = game_look(launcher.profile).title_face;
    let title = ui.painter().layout_job(game_title_job(
        launcher.profile.display_name,
        face,
        TOP_BAR_GARAMOND_TITLE_PX,
        TOP_BAR_CAPS_TITLE_PX,
        INK,
    ));
    let room_for_title = rect.right() - tabs_width - title_left;
    let title_column = TITLE_COLUMN_WIDTH_PX
        .min(room_for_title)
        .max(title.size().x + TITLE_TO_TABS_MIN_GAP_PX);
    let title_rect = Rect::from_min_max(
        pos2(title_left, rect.top()),
        pos2(
            title_left + title_column - TITLE_TO_TABS_MIN_GAP_PX / 2.0,
            rect.bottom(),
        ),
    );
    let title_top = middle - title.size().y / 2.0;
    ui.painter()
        .with_clip_rect(title_rect)
        .galley(pos2(title_left, title_top), title, INK);
    let tabs_rect = Rect::from_min_size(
        pos2(title_left + title_column, middle - TAB_HEIGHT_PX / 2.0),
        vec2(tabs_width, TAB_HEIGHT_PX),
    );
    ui.scope_builder(UiBuilder::new().max_rect(tabs_rect), |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = TAB_GAP_PX;
            for (value, label) in Tab::ALL {
                if tab_button(ui, label, *tab == value).clicked() {
                    *tab = value;
                }
            }
        });
    });
}

fn back_button(ui: &mut Ui, left: f32, middle: f32) -> Response {
    let galley = caps_galley(ui.painter(), "Home", INK);
    let width =
        BACK_TRIANGLE_WIDTH_PX + BACK_TRIANGLE_TO_TEXT_PX + galley.size().x + BACK_RIGHT_PADDING_PX;
    let rect = Rect::from_min_size(
        pos2(left, middle - BACK_DIVIDER_HEIGHT_PX / 2.0),
        vec2(width, BACK_DIVIDER_HEIGHT_PX),
    );
    let response = ui.interact(rect, Id::new("top-bar-back-home"), Sense::click());
    let color = INK;
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, 0.0, HOVER_WASH);
    }
    paint_triangle_left(painter, pos2(left, middle), color);
    let text_left = left + BACK_TRIANGLE_WIDTH_PX + BACK_TRIANGLE_TO_TEXT_PX;
    painter.galley(
        pos2(text_left, middle - galley.size().y / 2.0),
        galley,
        color,
    );
    painter.vline(
        rect.right() - 0.5,
        rect.y_range(),
        Stroke::new(1.0, HAIRLINE_STRONG),
    );
    response
}

fn tabs_width(ui: &Ui) -> f32 {
    let gaps = TAB_GAP_PX * (Tab::ALL.len() as f32 - 1.0);
    Tab::ALL
        .iter()
        .map(|(_, label)| {
            caps_galley(ui.painter(), label, INK).size().x + 2.0 * TAB_SIDE_PADDING_PX
        })
        .sum::<f32>()
        + gaps
}

fn tab_button(ui: &mut Ui, label: &str, selected: bool) -> Response {
    let galley = caps_galley(ui.painter(), label, INK);
    let size = vec2(galley.size().x + 2.0 * TAB_SIDE_PADDING_PX, TAB_HEIGHT_PX);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();
    let text_at = rect.center() - galley.size() / 2.0;
    if selected {
        painter.rect_filled(
            rect.translate(vec2(TAB_SHADOW_OFFSET_PX, TAB_SHADOW_OFFSET_PX)),
            0.0,
            HARD_SHADOW,
        );
        painter.rect_filled(rect, 0.0, SAND);
        painter.galley_with_override_text_color(text_at, galley, INK_ON_SAND);
    } else {
        let color = if response.hovered() { INK } else { INK_MUTED };
        painter.galley_with_override_text_color(text_at, galley, color);
    }
    response
}
