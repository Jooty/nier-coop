use hudhook::imgui::{Condition, Ui};
use rc_proto::StoryJumpPoint;

use crate::actions::{self, Action};
use crate::story_jump::{host_can_offer_jump, jump_is_waiting_for_menu, last_refusal};

#[derive(Debug, Default)]
pub(super) struct StoryJumpPanel {
    queue_status: String,
}

impl StoryJumpPanel {
    pub(super) fn draw(&mut self, ui: &Ui, open: &mut bool) {
        if !*open {
            return;
        }
        ui.window("story jump")
            .opened(open)
            .position([372.0, 332.0], Condition::FirstUseEver)
            .size([320.0, 170.0], Condition::FirstUseEver)
            .build(|| {
                if !host_can_offer_jump() {
                    ui.text("only a connected host can jump");
                    return;
                }
                if ui.button("Skip the prologue") {
                    self.queue_status =
                        match actions::enqueue(Action::StoryJump(StoryJumpPoint::AfterPrologue)) {
                            Ok(()) => String::from("asked the game to jump"),
                            Err(error) => format!("not queued: {error}"),
                        };
                }
                if ui.button("Library: talk to Popola") {
                    self.queue_status =
                        match actions::enqueue(Action::StoryJump(StoryJumpPoint::AtPopolaTalk)) {
                            Ok(()) => String::from("asked the game to jump"),
                            Err(error) => format!("not queued: {error}"),
                        };
                }
                if !self.queue_status.is_empty() {
                    ui.text_wrapped(&self.queue_status);
                }
                if jump_is_waiting_for_menu() {
                    ui.text_wrapped("queued: close the game menu and the jump runs");
                }
                let refusal = last_refusal();
                if !refusal.is_empty() {
                    ui.text_wrapped(format!("refused: {refusal}"));
                }
            });
    }
}
