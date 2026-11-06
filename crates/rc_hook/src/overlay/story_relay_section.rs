use hudhook::imgui::Ui;

use crate::overlay::boss_messages_section::draw_boss_messages;
use crate::story_relay::{set_shared_story_events, shared_story_events};

pub(super) fn draw_story_relay(ui: &Ui) {
    ui.separator();
    let mut shared = shared_story_events();
    if ui.checkbox("Shared story events", &mut shared) {
        set_shared_story_events(shared);
    }
    draw_boss_messages(ui);
}
