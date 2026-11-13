use eframe::egui::text::LayoutJob;
use eframe::egui::{
    Align, Color32, FontId, Id, Layout, Rect, TextFormat, Ui, UiBuilder, pos2, vec2,
};
use nc_launcher_core::{ART_FADE_MS, Easing, ROW_ART_BRIGHTEN_MS, banner_facts};

use crate::app::motion::{changed_since, ease_toward, with_clock};
use crate::app::pages::banner_shapes::paint_banner_shapes;
use crate::app::pages::game_title::game_title_job;
use crate::app::pages::tone::tone_color;
use crate::app::palette::{INK, INK_MUTED, NOT_READY};
use crate::app::parts::{
    LineTone, caps_galley, command_button, copy_line, line_button, paint_square,
};
use crate::app::style::{body_font, garamond_family, small_font};
use crate::app::{Launcher, Tab, UpdateState, game_look};
use crate::home::{BannerArtState, CardView, GameCard, card_view};
use crate::update::short_commit;

use super::row_art::{RowArtLight, paint_row_art};

const ART_LEFT_OFFSET_PX: f32 = 276.0;
const TEXT_COLUMN_WIDTH_PX: f32 = 420.0;
const NUMBER_TOP_PX: f32 = 6.0;
const NUMBER_ROW_HEIGHT_PX: f32 = 16.0;
const NUMBER_SQUARE_SIDE_PX: f32 = 7.0;
const NUMBER_SQUARE_TO_TEXT_PX: f32 = 10.0;
const TITLE_TOP_PX: f32 = 28.0;
const GARAMOND_TITLE_PX: f32 = 44.0;
const CAPS_TITLE_PX: f32 = 28.0;
const TITLE_LINE_HEIGHT_PX: f32 = 46.0;
const TITLE_NOTE_PX: f32 = 18.0;
const TITLE_NOTE_LINE_HEIGHT_PX: f32 = 22.0;
const FACTS_TOP_PX: f32 = 106.0;
const FACT_KEY_WIDTH_PX: f32 = 80.0;
const FACT_ROW_HEIGHT_PX: f32 = 20.0;
const FACT_ROW_GAP_PX: f32 = 6.0;
const FOLDER_LINE_INSET_PX: f32 = 13.0;
const FOLDER_LINE_WIDTH_PX: f32 = 340.0;
const FOLDER_KEY_DROP_PX: f32 = 8.0;
const ACTIONS_BOTTOM_GAP_PX: f32 = 6.0;
const ACTION_HEIGHT_PX: f32 = 48.0;
const PLAY_SIZE_PX: (f32, f32) = (176.0, 48.0);
const ACTION_GAP_PX: f32 = 22.0;
const LINE_ACTION_SIDE_PADDING_PX: f32 = 20.0;
const NOTE_GAP_PX: f32 = 10.0;
const EMPTY_FACT: &str = "\u{2014}";
const ROW_ART_LOADED_SALT: &str = "row-art-loaded";
const ROW_ART_HOVER_SALT: &str = "row-art-hover";
const NOT_FOUND_NOTE: &str = "Install the game to play it here.";
const SWITCH_BLOCKED_NOTE: &str = "Finish the current game first.";
const SET_FOLDER_LABEL: &str = "Set the folder";
const PLAY_LABEL: &str = "Play";

pub(super) fn draw_game_row(
    ui: &mut Ui,
    rect: Rect,
    number: usize,
    tab: &mut Tab,
    launcher: &mut Launcher,
    card: &GameCard,
    switch_locked: bool,
) {
    let server_build = match &launcher.updater.state {
        UpdateState::Available(update) if update.plan.hook_for(card.profile.game_id).is_some() => {
            Some(short_commit(&update.manifest.commit).to_owned())
        }
        _ => None,
    };
    let view = card_view(card, server_build.as_deref());
    let current = card.profile.game_id == launcher.profile.game_id;
    let found = !view.greyed_out;
    paint_art(ui, rect, launcher, card, found);
    let text = Rect::from_min_size(
        rect.min,
        vec2(TEXT_COLUMN_WIDTH_PX.min(rect.width()), rect.height()),
    );
    paint_number(ui, text, number, current);
    paint_title(ui, text, card, &view, found);
    let facts_bottom = draw_facts(ui, text, &view, found);
    if !found {
        let note = ui
            .painter()
            .layout_no_wrap(NOT_FOUND_NOTE.to_owned(), body_font(), INK_MUTED);
        ui.painter().galley(
            pos2(text.left(), facts_bottom + NOTE_GAP_PX),
            note,
            INK_MUTED,
        );
    }
    let actions = Rect::from_min_size(
        pos2(
            text.left(),
            rect.bottom() - ACTIONS_BOTTOM_GAP_PX - ACTION_HEIGHT_PX,
        ),
        vec2(rect.width(), ACTION_HEIGHT_PX),
    );
    let blocked = !current && switch_locked;
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(actions)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing = vec2(ACTION_GAP_PX, 0.0);
            draw_actions(ui, tab, launcher, card, &view, current, blocked);
        },
    );
}

fn paint_art(ui: &Ui, rect: Rect, launcher: &mut Launcher, card: &GameCard, found: bool) {
    let art_rect = Rect::from_min_max(
        pos2(rect.left() + ART_LEFT_OFFSET_PX, rect.top()),
        rect.right_bottom(),
    );
    if art_rect.width() <= 0.0 {
        return;
    }
    let painter = ui.painter_at(art_rect);
    let game_id = card.profile.game_id;
    let state = launcher.banner_art.state(ui.ctx(), game_id);
    let ready = matches!(state, BannerArtState::Ready(_));
    let loaded_since = changed_since(ui, Id::unique((ROW_ART_LOADED_SALT, game_id)), ready);
    let pointed = if ui.rect_contains_pointer(rect) {
        1.0
    } else {
        0.0
    };
    let light = RowArtLight {
        shown: with_clock(ui, |clock, _| {
            clock.entrance(loaded_since, 0.0, ART_FADE_MS, Easing::EaseOut)
        }),
        brightened: ease_toward(
            ui,
            Id::unique((ROW_ART_HOVER_SALT, game_id)),
            pointed,
            ROW_ART_BRIGHTEN_MS,
            Easing::EaseOut,
        ),
    };
    match state {
        BannerArtState::Ready(art) => {
            let texture = if found { &art.colour } else { &art.grey };
            paint_row_art(
                &painter,
                art_rect,
                texture,
                banner_facts(game_id).focus,
                light,
            );
        }
        BannerArtState::Missing => {
            paint_banner_shapes(&painter, art_rect, game_look(card.profile).mark, INK_MUTED);
        }
        BannerArtState::Pending => {}
    }
}

fn paint_number(ui: &Ui, text: Rect, number: usize, current: bool) {
    let painter = ui.painter();
    let middle = text.top() + NUMBER_TOP_PX + NUMBER_ROW_HEIGHT_PX / 2.0;
    let mut left = text.left();
    if current {
        paint_square(
            painter,
            pos2(left + NUMBER_SQUARE_SIDE_PX / 2.0, middle),
            NUMBER_SQUARE_SIDE_PX,
            INK,
        );
        left += NUMBER_SQUARE_SIDE_PX + NUMBER_SQUARE_TO_TEXT_PX;
    }
    let galley = caps_galley(painter, &format!("{number:02}"), INK_MUTED);
    painter.galley(
        pos2(left, middle - galley.size().y / 2.0),
        galley,
        INK_MUTED,
    );
}

fn paint_title(ui: &Ui, text: Rect, card: &GameCard, view: &CardView, found: bool) {
    let painter = ui.painter_at(text);
    let color = if found { INK } else { INK_MUTED };
    let job = game_title_job(
        view.title,
        game_look(card.profile).title_face,
        GARAMOND_TITLE_PX,
        CAPS_TITLE_PX,
        color,
    );
    let title = painter.layout_job(job);
    let title_top = text.top() + TITLE_TOP_PX + (TITLE_LINE_HEIGHT_PX - title.size().y) / 2.0;
    painter.galley(pos2(text.left(), title_top), title, color);
    if let Some(note) = view.title_note {
        let format = TextFormat {
            font_id: FontId::new(TITLE_NOTE_PX, garamond_family()),
            color: INK_MUTED,
            italics: true,
            ..TextFormat::default()
        };
        let note = painter.layout_job(LayoutJob::single_section(note.to_owned(), format));
        let note_top = text.top()
            + TITLE_TOP_PX
            + TITLE_LINE_HEIGHT_PX
            + (TITLE_NOTE_LINE_HEIGHT_PX - note.size().y) / 2.0;
        painter.galley(pos2(text.left(), note_top), note, INK_MUTED);
    }
}

fn draw_facts(ui: &mut Ui, text: Rect, view: &CardView, found: bool) -> f32 {
    let mut top = text.top() + FACTS_TOP_PX;
    let value_left = text.left() + FACT_KEY_WIDTH_PX;
    match (&view.folder_display, found) {
        (Some(folder), true) => {
            paint_key(
                ui,
                "Folder",
                text.left(),
                top + FOLDER_KEY_DROP_PX,
                INK_MUTED,
            );
            let line = Rect::from_min_size(
                pos2(value_left - FOLDER_LINE_INSET_PX, top),
                vec2(FOLDER_LINE_WIDTH_PX, text.height()),
            );
            let used = ui
                .scope_builder(UiBuilder::new().max_rect(line), |ui| {
                    copy_line(
                        ui,
                        view.title,
                        folder,
                        &view.found_text,
                        FOLDER_LINE_WIDTH_PX,
                        small_font(),
                    )
                    .rect
                    .height()
                })
                .inner;
            top += used + FACT_ROW_GAP_PX;
        }
        _ => {
            let shown = if view.found_text.is_empty() {
                EMPTY_FACT
            } else {
                &view.found_text
            };
            fact_row(ui, text, top, "Folder", shown, INK, INK_MUTED);
            top += FACT_ROW_HEIGHT_PX + FACT_ROW_GAP_PX;
        }
    }
    let rows: Vec<(&str, &str, Color32, Color32)> = if found {
        let mut rows = vec![(
            "Mod",
            view.mod_text.as_str(),
            tone_color(view.mod_tone),
            INK_MUTED,
        )];
        if let Some(update) = &view.update_text {
            rows.push(("Update", update.as_str(), NOT_READY, NOT_READY));
        }
        if let Some(saves) = &view.saves_text {
            rows.push(("Saves", saves.as_str(), INK, INK_MUTED));
        }
        rows
    } else {
        vec![
            ("Mod", EMPTY_FACT, INK_MUTED, INK_MUTED),
            ("Saves", EMPTY_FACT, INK_MUTED, INK_MUTED),
        ]
    };
    for (key, value, value_color, key_color) in rows {
        fact_row(ui, text, top, key, value, value_color, key_color);
        top += FACT_ROW_HEIGHT_PX + FACT_ROW_GAP_PX;
    }
    top - FACT_ROW_GAP_PX
}

fn fact_row(
    ui: &Ui,
    text: Rect,
    top: f32,
    key: &str,
    value: &str,
    value_color: Color32,
    key_color: Color32,
) {
    let middle = top + FACT_ROW_HEIGHT_PX / 2.0;
    let key_galley = caps_galley(ui.painter(), key, key_color);
    let key_top = middle - key_galley.size().y / 2.0;
    ui.painter()
        .galley(pos2(text.left(), key_top), key_galley, key_color);
    let value_left = text.left() + FACT_KEY_WIDTH_PX;
    let painter = ui.painter_at(Rect::from_min_max(
        pos2(value_left, top),
        pos2(text.right(), top + FACT_ROW_HEIGHT_PX),
    ));
    let galley = painter.layout_no_wrap(value.to_owned(), body_font(), value_color);
    painter.galley(
        pos2(value_left, middle - galley.size().y / 2.0),
        galley,
        value_color,
    );
}

fn paint_key(ui: &Ui, key: &str, left: f32, top: f32, color: Color32) {
    let galley = caps_galley(ui.painter(), key, color);
    ui.painter().galley(pos2(left, top), galley, color);
}

fn draw_actions(
    ui: &mut Ui,
    tab: &mut Tab,
    launcher: &mut Launcher,
    card: &GameCard,
    view: &CardView,
    current: bool,
    blocked: bool,
) {
    if !view.play_enabled {
        let set_folder = line_button(
            ui,
            SET_FOLDER_LABEL,
            LineTone::Plain,
            ACTION_HEIGHT_PX,
            Some(LINE_ACTION_SIDE_PADDING_PX),
            !blocked,
        );
        if set_folder.clicked() {
            launcher.open_game(card.profile);
            *tab = Tab::Settings;
        }
        return;
    }
    let mut play = command_button(ui, PLAY_LABEL, PLAY_SIZE_PX, current, !blocked);
    if blocked {
        play = play.on_hover_text(SWITCH_BLOCKED_NOTE);
    }
    if play.clicked() {
        launcher.open_game(card.profile);
    }
    if let Some(label) = view.mod_action {
        let enabled = !blocked && !launcher.busy();
        let action = line_button(
            ui,
            label,
            LineTone::Warm,
            ACTION_HEIGHT_PX,
            Some(LINE_ACTION_SIDE_PADDING_PX),
            enabled,
        );
        if action.clicked() {
            if view.update_from_server {
                launcher.start_update_apply(ui.ctx(), true);
            } else {
                launcher.install_mod_from_home(card.profile, ui.ctx());
            }
        }
    }
}
