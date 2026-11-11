use std::sync::atomic::{AtomicBool, Ordering};

use hudhook::imgui::Ui;

use crate::hud::game_log_hook::game_log_is_hooked;
use crate::hud::minimap_dot::draw_minimap_tuning;

static NAME_TAG_SHOWN: AtomicBool = AtomicBool::new(true);
static DISTANCE_SHOWN: AtomicBool = AtomicBool::new(false);
static MINIMAP_DOT_SHOWN: AtomicBool = AtomicBool::new(true);
static CO_OP_LINES_SHOWN: AtomicBool = AtomicBool::new(true);
static GAME_LOG_USED: AtomicBool = AtomicBool::new(true);

pub(super) fn name_tag_shown() -> bool {
    NAME_TAG_SHOWN.load(Ordering::Relaxed)
}

pub(super) fn distance_shown() -> bool {
    DISTANCE_SHOWN.load(Ordering::Relaxed)
}

pub(super) fn minimap_dot_shown() -> bool {
    MINIMAP_DOT_SHOWN.load(Ordering::Relaxed)
}

pub(super) fn co_op_lines_shown() -> bool {
    CO_OP_LINES_SHOWN.load(Ordering::Relaxed)
}

pub(super) fn game_log_used() -> bool {
    GAME_LOG_USED.load(Ordering::Relaxed)
}

pub(crate) fn draw_hud_settings(ui: &Ui) {
    checkbox(ui, "Show the partner's name", &NAME_TAG_SHOWN);
    checkbox(ui, "Show the distance with the name", &DISTANCE_SHOWN);
    checkbox(ui, "Show the partner on the minimap", &MINIMAP_DOT_SHOWN);
    checkbox(ui, "Show co-op lines", &CO_OP_LINES_SHOWN);
    if game_log_is_hooked() {
        checkbox(ui, "Co-op lines go to the game's own log", &GAME_LOG_USED);
    } else {
        ui.text_disabled("Co-op lines are drawn by the overlay: the game's own log is not hooked.");
    }
    draw_minimap_tuning(ui);
    ui.text_disabled("These last until the game closes.");
}

fn checkbox(ui: &Ui, label: &str, setting: &AtomicBool) {
    let mut value = setting.load(Ordering::Relaxed);
    if ui.checkbox(label, &mut value) {
        setting.store(value, Ordering::Relaxed);
    }
}
