use hudhook::imgui::Ui;

use crate::boss_messages::boss_message_counts;

pub(super) fn draw_boss_messages(ui: &Ui) {
    let counts = boss_message_counts();
    ui.text(format!(
        "Boss messages: sent {}, raised {}, skipped {}",
        counts.sent, counts.raised, counts.skipped
    ));
}
