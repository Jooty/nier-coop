use hudhook::imgui::{Condition, Ui};
use replicant_game::offsets::MAX_PHASE_POINT;
use replicant_game::{PHASE_TABLE, PhaseEntry, Replicant};

use crate::actions::{self, Action};
use crate::build_gate::{GateError, VerifiedBuild};
use crate::memory::InProcessMemory;

const LIST_HEIGHT: f32 = 280.0;

#[derive(Debug, Default)]
pub(super) struct LevelsPanel {
    filter: String,
    point: i32,
    status: String,
}

impl LevelsPanel {
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
        ui.window("levels")
            .opened(open)
            .position([372.0, 16.0], Condition::FirstUseEver)
            .size([380.0, 440.0], Condition::FirstUseEver)
            .build(|| {
                let Some(game) = game else {
                    ui.text(match gate {
                        Ok(_) => "no game memory",
                        Err(_) => "build gate failed: level select is off",
                    });
                    return;
                };
                self.draw_controls(ui, game);
                self.draw_list(ui, game);
            });
    }

    fn draw_controls(&mut self, ui: &Ui, game: &Replicant<InProcessMemory>) {
        match game.current_phase() {
            Ok(phase) => ui.text(format!("current: {phase}")),
            Err(error) => ui.text(format!("current: {error}")),
        }
        ui.input_text("filter", &mut self.filter).build();
        ui.input_int("point", &mut self.point).build();
        self.point = self.point.clamp(0, MAX_PHASE_POINT);
        if !actions::is_draining() {
            ui.text("game-thread hook not installed: selecting does nothing");
        }
        if !self.status.is_empty() {
            ui.text_wrapped(&self.status);
        }
    }

    fn draw_list(&mut self, ui: &Ui, game: &Replicant<InProcessMemory>) {
        let current = game.current_phase().unwrap_or_default();
        let filter = self.filter.to_ascii_lowercase();
        let mut chosen: Option<&'static PhaseEntry> = None;
        ui.child_window("phase list")
            .size([0.0, LIST_HEIGHT])
            .border(true)
            .build(|| {
                for entry in PHASE_TABLE
                    .iter()
                    .filter(|entry| matches_filter(entry, &filter))
                {
                    let label = format!("{}  ({})", entry.name, entry.area);
                    if ui
                        .selectable_config(label)
                        .selected(entry.name == current)
                        .build()
                    {
                        chosen = Some(entry);
                    }
                }
            });
        if let Some(entry) = chosen {
            self.status = queue_change(entry, self.point);
        }
    }
}

fn matches_filter(entry: &PhaseEntry, lowercase_filter: &str) -> bool {
    entry.name.to_ascii_lowercase().contains(lowercase_filter)
        || entry.area.to_ascii_lowercase().contains(lowercase_filter)
}

fn queue_change(entry: &PhaseEntry, point: i32) -> String {
    let queued = Action::change_phase(entry.name, point)
        .map_err(|error| error.to_string())
        .and_then(|action| actions::enqueue(action).map_err(|error| error.to_string()));
    match queued {
        Ok(()) => format!("queued {} point {point}", entry.name),
        Err(error) => format!("not queued: {error}"),
    }
}
