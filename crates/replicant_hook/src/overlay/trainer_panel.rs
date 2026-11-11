use hudhook::imgui::{Condition, Ui};
use replicant_game::Replicant;

use crate::actions::{self, Action};
use crate::build_gate::{GateError, VerifiedBuild};
use crate::game_hooks;
use crate::memory::InProcessMemory;

const GOLD_PER_BUTTON_PRESS: i32 = 1000;

#[derive(Debug, Default)]
pub(super) struct TrainerPanel {
    status: String,
}

impl TrainerPanel {
    pub(super) fn draw(
        &mut self,
        ui: &Ui,
        gate: &Result<VerifiedBuild, GateError>,
        game: Option<&Replicant<InProcessMemory>>,
        open: &mut bool,
    ) {
        if !*open {
            return;
        }
        ui.window("trainer")
            .opened(open)
            .position([16.0, 332.0], Condition::FirstUseEver)
            .size([340.0, 200.0], Condition::FirstUseEver)
            .build(|| {
                let Some(game) = game else {
                    ui.text(match gate {
                        Ok(_) => "no game memory",
                        Err(_) => "build gate failed: trainer is off",
                    });
                    return;
                };
                if !actions::is_draining() {
                    ui.text("game-thread hook not installed: toggles do nothing");
                }
                self.draw_toggles(ui, game);
                self.draw_buttons(ui);
                if !self.status.is_empty() {
                    ui.text_wrapped(&self.status);
                }
            });
    }

    fn draw_toggles(&mut self, ui: &Ui, game: &Replicant<InProcessMemory>) {
        let mut invincible = game_hooks::invincibility_enabled();
        if ui.checkbox("invincible (hp never drops)", &mut invincible) {
            self.queue(Action::SetInfiniteHealth {
                enabled: invincible,
            });
        }
        if !game_hooks::invincibility_installed() {
            ui.text_disabled("invincible: SetHp hook not installed");
        }
        let mut infinite_magic = game.infinite_magic_enabled().unwrap_or(false);
        if ui.checkbox("infinite mp", &mut infinite_magic) {
            self.queue(Action::SetInfiniteMagic {
                enabled: infinite_magic,
            });
        }
        let mut noclip = game_hooks::noclip_enabled();
        if ui.checkbox(
            "noclip (WASD, Space up, Ctrl down, Shift fast)",
            &mut noclip,
        ) {
            self.queue(Action::SetNoclip { enabled: noclip });
        }
        if !game_hooks::noclip_installed() {
            ui.text_disabled("noclip: move step hooks not installed");
        }
    }

    fn draw_buttons(&mut self, ui: &Ui) {
        if ui.button("refill hp") {
            self.queue(Action::RefillHealth);
        }
        ui.same_line();
        if ui.button("refill mp") {
            self.queue(Action::RefillMagic);
        }
        ui.same_line();
        if ui.button(format!("+{GOLD_PER_BUTTON_PRESS} gold")) {
            match Action::add_gold(GOLD_PER_BUTTON_PRESS) {
                Ok(action) => self.queue(action),
                Err(error) => self.status = error.to_string(),
            }
        }
    }

    fn queue(&mut self, action: Action) {
        self.status = match actions::enqueue(action) {
            Ok(()) => format!("queued {action:?}"),
            Err(error) => format!("not queued: {error}"),
        };
    }
}
