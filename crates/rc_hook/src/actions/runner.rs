use crate::actions::action::Action;
use crate::actions::phase_change::change_phase;
use crate::actions::queue::take_all;
use crate::actions::trainer::run_trainer_action;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::puppet::{despawn, set_mirror_delay_ms, spawn_mirror, tick as tick_puppets, tick_ghost};

pub(super) fn run_queued_actions(build: &VerifiedBuild) {
    for action in take_all() {
        let outcome = match action {
            Action::ChangePhase { phase, point } => {
                change_phase(build, phase, point).map_err(|error| error.to_string())
            }
            Action::PuppetSpawn { delay_ms } => {
                set_mirror_delay_ms(delay_ms);
                spawn_mirror(build)
                    .map(|_| ())
                    .map_err(|error| error.to_string())
            }
            Action::PuppetDespawn { id } => despawn(build, id)
                .map(|_| ())
                .map_err(|error| error.to_string()),
            other => run_trainer_action(build, other).map_err(|error| error.to_string()),
        };
        match outcome {
            Ok(()) => log_line(&format!("action: {action:?}")),
            Err(error) => log_line(&format!("action: {action:?} failed ({error})")),
        }
    }
    tick_puppets(build);
    tick_ghost(build);
}
