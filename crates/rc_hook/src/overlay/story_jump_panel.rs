use hudhook::imgui::{Condition, Ui};
use rc_proto::StoryJumpPoint;

use crate::actions::{self, Action};
use crate::story_jump::{host_can_offer_jump, jump_is_waiting_for_menu, last_refusal};
use crate::teleport::last_outcome as last_teleport_outcome;

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
            .size([320.0, 290.0], Condition::FirstUseEver)
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
                if ui.button("Village: before the next day") {
                    self.queue_status =
                        match actions::enqueue(Action::StoryJump(StoryJumpPoint::BeforeNextDay)) {
                            Ok(()) => String::from("asked the game to jump"),
                            Err(error) => format!("not queued: {error}"),
                        };
                }
                if ui.button("Lost Shrine: the entrance") {
                    self.queue_status =
                        match actions::enqueue(Action::StoryJump(StoryJumpPoint::AtShrineEntrance))
                        {
                            Ok(()) => String::from("asked the game to jump"),
                            Err(error) => format!("not queued: {error}"),
                        };
                }
                if ui.button("Shrine top: the stone field") {
                    self.queue_status =
                        match actions::enqueue(Action::StoryJump(StoryJumpPoint::AtShrineTop)) {
                            Ok(()) => String::from("asked the game to jump"),
                            Err(error) => format!("not queued: {error}"),
                        };
                }
                if ui.button("Shrine boss: the arena") {
                    self.queue_status =
                        match actions::enqueue(Action::StoryJump(StoryJumpPoint::AtShrineBoss)) {
                            Ok(()) => String::from("asked the game to jump"),
                            Err(error) => format!("not queued: {error}"),
                        };
                }
                ui.separator();
                if ui.button("Bring the guest here") {
                    self.queue_status = match actions::enqueue(Action::BringGuestHere) {
                        Ok(()) => String::from("asked the game to bring the guest"),
                        Err(error) => format!("not queued: {error}"),
                    };
                }
                if ui.button("Go to the guest") {
                    self.queue_status = match actions::enqueue(Action::GoToGuest) {
                        Ok(()) => String::from("asked the game to go to the guest"),
                        Err(error) => format!("not queued: {error}"),
                    };
                }
                let teleport_outcome = last_teleport_outcome();
                if !teleport_outcome.is_empty() {
                    ui.text_wrapped(teleport_outcome);
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
