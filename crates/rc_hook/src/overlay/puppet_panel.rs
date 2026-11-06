use hudhook::imgui::{Condition, Ui};
use rc_game::puppet::MAX_MIRROR_DELAY_MS;

use crate::actions::{self, Action};
use crate::build_gate::{GateError, VerifiedBuild};
use crate::puppet;

#[derive(Debug, Default)]
pub(super) struct PuppetPanel {
    status: String,
    delay_ms: i32,
    delay_loaded: bool,
}

impl PuppetPanel {
    pub(super) fn draw(
        &mut self,
        ui: &Ui,
        gate: &Result<VerifiedBuild, GateError>,
        open: &mut bool,
    ) {
        if !*open {
            return;
        }
        ui.window("puppet")
            .opened(open)
            .position([372.0, 472.0], Condition::FirstUseEver)
            .size([380.0, 300.0], Condition::FirstUseEver)
            .build(|| {
                if gate.is_err() {
                    ui.text("build gate failed: puppets are off");
                    return;
                }
                if !actions::is_draining() {
                    ui.text("game-thread hook not installed: buttons do nothing");
                }
                self.draw_controls(ui);
                draw_puppets(ui);
            });
    }

    fn draw_controls(&mut self, ui: &Ui) {
        if !self.delay_loaded {
            self.delay_ms = i32::from(puppet::mirror_delay_ms());
            self.delay_loaded = true;
        }
        if ui.slider(
            "mirror delay (ms)",
            0,
            i32::from(MAX_MIRROR_DELAY_MS),
            &mut self.delay_ms,
        ) {
            puppet::set_mirror_delay_ms(u16::try_from(self.delay_ms).unwrap_or(0));
        }
        if ui.button("spawn mirror puppet") {
            self.queue(Action::puppet_spawn(self.delay_ms));
        }
        ui.same_line();
        if ui.button("despawn all") {
            self.queue(Ok(Action::PuppetDespawn { id: None }));
        }
        if !self.status.is_empty() {
            ui.text_wrapped(&self.status);
        }
    }

    fn queue(&mut self, action: Result<Action, actions::ActionError>) {
        self.status = match action {
            Ok(action) => match actions::enqueue(action) {
                Ok(()) => format!("queued {action:?}"),
                Err(error) => format!("not queued: {error}"),
            },
            Err(error) => format!("not queued: {error}"),
        };
    }
}

fn draw_puppets(ui: &Ui) {
    ui.separator();
    let puppets = puppet::snapshot();
    if puppets.is_empty() {
        ui.text("no puppets");
        return;
    }
    for info in puppets {
        ui.text(format!("puppet {} actor {:#x}", info.id, info.actor));
        if let Some([x, y, z]) = info.position {
            ui.text(format!("  position: {x:.2} {y:.2} {z:.2}"));
        }
        ui.text(format!(
            "  motion: {} index {:?} frame {:?}",
            info.motion_name.as_deref().unwrap_or("?"),
            info.motion_index,
            info.frame.map(|frame| (frame * 10.0).round() / 10.0),
        ));
        if let Some(error) = &info.last_error {
            ui.text_wrapped(format!("  error: {error}"));
        }
    }
}
