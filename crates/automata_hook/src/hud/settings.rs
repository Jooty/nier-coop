use std::sync::atomic::{AtomicBool, Ordering};

use hudhook::imgui::Ui;

static NAME_TAG_SHOWN: AtomicBool = AtomicBool::new(true);
static DISTANCE_SHOWN: AtomicBool = AtomicBool::new(false);

pub(super) fn name_tag_shown() -> bool {
    NAME_TAG_SHOWN.load(Ordering::Relaxed)
}

pub(super) fn distance_shown() -> bool {
    DISTANCE_SHOWN.load(Ordering::Relaxed)
}

pub(crate) fn draw_hud_settings(ui: &Ui) {
    checkbox(ui, "Show the partner's name", &NAME_TAG_SHOWN);
    checkbox(ui, "Show the distance with the name", &DISTANCE_SHOWN);
    ui.text_disabled("These last until the game closes.");
}

fn checkbox(ui: &Ui, label: &str, setting: &AtomicBool) {
    let mut value = setting.load(Ordering::Relaxed);
    if ui.checkbox(label, &mut value) {
        setting.store(value, Ordering::Relaxed);
    }
}
