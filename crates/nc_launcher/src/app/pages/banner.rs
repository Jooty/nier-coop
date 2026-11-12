use eframe::egui::{
    Align, Align2, Color32, CornerRadius, FontId, Label, Layout, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, UiBuilder, pos2, vec2,
};
use nc_launcher_core::banner_facts;

use crate::app::pages::banner_art::paint_banner_art;
use crate::app::pages::banner_shapes::paint_banner_shapes;
use crate::app::pages::big_button::{BIG_BUTTON_HEIGHT_PX, outline_button, play_button};
use crate::app::pages::marks::status_dot;
use crate::app::pages::tone::tone_color;
use crate::app::palette::{LINE_SOFT, MUTED, SURFACE, TEXT, WARN};
use crate::app::style::{heading_font_family, small_font};
use crate::app::{Launcher, game_look};
use crate::home::{BannerArtState, CardTone, GameCard, card_view};

pub(super) const BANNER_CORNER_RADIUS_PX: u8 = 12;
const BANNER_LEFT_PADDING_PX: f32 = 30.0;
const BANNER_RIGHT_PADDING_PX: f32 = 28.0;
const BANNER_VERTICAL_PADDING_PX: f32 = 22.0;
const TEXT_COLUMN_SHARE: f32 = 0.55;
const MIN_TEXT_COLUMN_WIDTH_PX: f32 = 470.0;
const MAX_TEXT_COLUMN_WIDTH_PX: f32 = 620.0;
const TAG_HEIGHT_PX: f32 = 22.0;
const TAG_SIDE_PADDING_PX: f32 = 7.0;
const TAG_GAP_PX: f32 = 6.0;
const TAG_CORNER_RADIUS_PX: u8 = 4;
const TAG_BORDER_ALPHA: f32 = 0.55;
const TAGS_TO_TITLE_GAP_PX: f32 = 8.0;
const BANNER_TITLE_SIZE_PX: f32 = 44.0;
const TITLE_NOTE_SIZE_PX: f32 = 22.0;
const TITLE_NOTE_GAP_PX: f32 = 10.0;
const TITLE_TO_FACTS_GAP_PX: f32 = 10.0;
const FACT_KEY_WIDTH_PX: f32 = 52.0;
const FACT_KEY_GAP_PX: f32 = 12.0;
const FACT_ROW_HEIGHT_PX: f32 = 19.0;
const FACT_ROW_GAP_PX: f32 = 5.0;
const FACT_DOT_GAP_PX: f32 = 8.0;
const FACTS_TO_BUTTONS_MIN_GAP_PX: f32 = 16.0;
const BUTTON_GAP_PX: f32 = 10.0;
const NOT_FOUND_NOTE: &str = "Install the game to play it here.";
const SWITCH_BLOCKED_NOTE: &str = "Finish the current game first.";

pub(super) fn draw_banner(
    ui: &mut Ui,
    rect: Rect,
    launcher: &mut Launcher,
    card: &GameCard,
    switch_locked: bool,
) {
    let view = card_view(card);
    let look = game_look(card.profile);
    let painter = ui.painter_at(rect);
    let radius = CornerRadius::same(BANNER_CORNER_RADIUS_PX);
    painter.rect_filled(rect, radius, SURFACE);
    let text_column_width = (rect.width() * TEXT_COLUMN_SHARE)
        .clamp(MIN_TEXT_COLUMN_WIDTH_PX, MAX_TEXT_COLUMN_WIDTH_PX)
        .min(rect.width());
    let art_start_x = rect.left() + text_column_width;
    match launcher.banner_art.state(ui.ctx(), card.profile.game_id) {
        BannerArtState::Ready(art) => paint_banner_art(
            &painter,
            rect,
            BANNER_CORNER_RADIUS_PX,
            art_start_x,
            &art,
            banner_facts(card.profile.game_id).focus,
        ),
        BannerArtState::Missing => paint_banner_shapes(&painter, rect, look.mark, look.accent),
        BannerArtState::Pending => {}
    }
    painter.rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, LINE_SOFT),
        StrokeKind::Inside,
    );

    let content = Rect::from_min_max(
        pos2(
            rect.left() + BANNER_LEFT_PADDING_PX,
            rect.top() + BANNER_VERTICAL_PADDING_PX,
        ),
        pos2(
            art_start_x - BANNER_RIGHT_PADDING_PX,
            rect.bottom() - BANNER_VERTICAL_PADDING_PX,
        ),
    );
    let facts_bottom = ui
        .scope_builder(
            UiBuilder::new()
                .max_rect(content)
                .layout(Layout::top_down(Align::Min)),
            |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                tags_row(ui, view.tags, look.accent);
                ui.add_space(TAGS_TO_TITLE_GAP_PX);
                title_row(ui, view.title, view.title_note);
                ui.add_space(TITLE_TO_FACTS_GAP_PX);
                let found_color = match view.found_tone {
                    CardTone::Good => TEXT,
                    tone => tone_color(tone),
                };
                fact_row(ui, "Found", None, &view.found_text, found_color);
                fact_row(
                    ui,
                    "Mod",
                    Some(tone_color(view.mod_tone)),
                    &view.mod_text,
                    mod_text_color(view.mod_tone),
                );
                if let Some(saves) = &view.saves_text {
                    fact_row(ui, "Saves", None, saves, TEXT);
                }
            },
        )
        .response
        .rect
        .bottom();

    let buttons_top =
        (content.bottom() - BIG_BUTTON_HEIGHT_PX).max(facts_bottom + FACTS_TO_BUTTONS_MIN_GAP_PX);
    let buttons = Rect::from_min_size(
        pos2(content.left(), buttons_top),
        vec2(content.width(), BIG_BUTTON_HEIGHT_PX),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(buttons)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(BUTTON_GAP_PX, 0.0);
            let is_active = card.profile.game_id == launcher.profile.game_id;
            let blocked = !is_active && switch_locked;
            let mut play = play_button(ui, look.accent, view.play_enabled && !blocked);
            if blocked {
                play = play.on_hover_text(SWITCH_BLOCKED_NOTE);
            }
            if play.clicked() {
                launcher.open_game(card.profile);
            }
            if let Some(label) = view.mod_action {
                let enabled = !blocked && !launcher.busy();
                if outline_button(ui, label, WARN, enabled).clicked() {
                    launcher.install_mod_from_home(card.profile, ui.ctx());
                }
            }
            if view.greyed_out {
                ui.add(
                    Label::new(RichText::new(NOT_FOUND_NOTE).color(MUTED))
                        .selectable(false)
                        .truncate(),
                );
            }
        },
    );
}

fn mod_text_color(tone: CardTone) -> Color32 {
    match tone {
        CardTone::Good => TEXT,
        tone => tone_color(tone),
    }
}

fn tags_row(ui: &mut Ui, tags: &[&str], accent: Color32) {
    if tags.is_empty() {
        return;
    }
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TAG_GAP_PX;
        for tag in tags {
            let galley = ui
                .painter()
                .layout_no_wrap((*tag).to_owned(), small_font(), accent);
            let size = vec2(galley.size().x + 2.0 * TAG_SIDE_PADDING_PX, TAG_HEIGHT_PX);
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(TAG_CORNER_RADIUS_PX),
                Stroke::new(1.0, accent.gamma_multiply(TAG_BORDER_ALPHA)),
                StrokeKind::Inside,
            );
            ui.painter()
                .galley(rect.center() - galley.size() / 2.0, galley, accent);
        }
    });
}

fn title_row(ui: &mut Ui, title: &str, note: Option<&str>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = TITLE_NOTE_GAP_PX;
        ui.add(
            Label::new(
                RichText::new(title)
                    .font(FontId::new(BANNER_TITLE_SIZE_PX, heading_font_family()))
                    .color(TEXT),
            )
            .selectable(false)
            .extend(),
        );
        if let Some(note) = note {
            ui.with_layout(Layout::left_to_right(Align::BOTTOM), |ui| {
                ui.add(
                    Label::new(
                        RichText::new(note)
                            .font(FontId::new(TITLE_NOTE_SIZE_PX, heading_font_family()))
                            .color(MUTED),
                    )
                    .selectable(false)
                    .truncate(),
                );
            });
        }
    });
}

fn fact_row(ui: &mut Ui, key: &str, dot: Option<Color32>, value: &str, value_color: Color32) {
    ui.add_space(FACT_ROW_GAP_PX);
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), FACT_ROW_HEIGHT_PX),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.set_height(FACT_ROW_HEIGHT_PX);
            ui.spacing_mut().item_spacing.x = 0.0;
            let (key_rect, _) =
                ui.allocate_exact_size(vec2(FACT_KEY_WIDTH_PX, FACT_ROW_HEIGHT_PX), Sense::hover());
            ui.painter().text(
                pos2(key_rect.left(), key_rect.center().y),
                Align2::LEFT_CENTER,
                key,
                small_font(),
                MUTED,
            );
            ui.add_space(FACT_KEY_GAP_PX);
            if let Some(color) = dot {
                status_dot(ui, color);
                ui.add_space(FACT_DOT_GAP_PX);
            }
            ui.add(
                Label::new(RichText::new(value).color(value_color))
                    .selectable(false)
                    .truncate(),
            );
        },
    );
}
