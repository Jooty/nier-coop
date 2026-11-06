use crate::actions::action::Action;
use crate::actions::phase_change::change_phase;
use crate::actions::queue::take_all;
use crate::actions::trainer::run_trainer_action;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::dialogue::tick_dialogue;
use crate::door_sync::tick as tick_door_sync;
use crate::downed::tick_downed;
use crate::effects::tick as tick_effects;
use crate::fade_sync::tick as tick_fade_sync;
use crate::music::tick as tick_music;
use crate::pickups::tick as tick_pickups;
use crate::puppet::{tick as tick_puppets, tick_ghost};
use crate::stay_together::tick as tick_stay_together;

pub(super) fn run_queued_actions(build: &VerifiedBuild) {
    for action in take_all() {
        let outcome = match action {
            Action::ChangePhase { phase, point } => {
                change_phase(build, phase, point).map_err(|error| error.to_string())
            }
            other => run_trainer_action(build, other).map_err(|error| error.to_string()),
        };
        match outcome {
            Ok(()) => log_line(&format!("action: {action:?}")),
            Err(error) => log_line(&format!("action: {action:?} failed ({error})")),
        }
    }
    tick_puppets(build);
    tick_ghost(build);
    tick_effects(build);
    tick_music(build);
    tick_downed(build);
    tick_pickups(build);
    tick_stay_together(build);
    tick_dialogue(build);
    tick_fade_sync(build);
    tick_door_sync(build);
}
