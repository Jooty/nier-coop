use crate::actions::action::Action;
use crate::actions::phase_change::change_phase;
use crate::actions::queue::take_all;
use crate::actions::trainer::run_trainer_action;
use crate::boss_messages::tick as tick_boss_messages;
use crate::build_gate::VerifiedBuild;
use crate::bullets::{tick_own_bullets, tick_partner_bullets};
use crate::containers::tick as tick_containers;
use crate::cutscene_skip::tick as tick_cutscene_skip;
use crate::diagnostics::log_line;
use crate::dialogue::tick_dialogue;
use crate::door_sync::tick as tick_door_sync;
use crate::downed::tick_downed;
use crate::effects::tick as tick_effects;
use crate::enemy_bullets::tick as tick_enemy_bullets;
use crate::fade_sync::tick as tick_fade_sync;
use crate::fight_sync::tick as tick_fight_sync;
use crate::music::tick as tick_music;
use crate::one_sided::tick as tick_one_sided;
use crate::pickups::tick as tick_pickups;
use crate::prologue_levels::tick as tick_prologue_levels;
use crate::puppet::{tick as tick_puppets, tick_ghost, tick_own_powers, tick_partner_powers};
use crate::script_rolls::tick as tick_script_rolls;
use crate::stay_together::tick as tick_stay_together;
use crate::story_jump::{run_story_jump, tick as tick_story_jump};
use crate::story_progress::tick as tick_story_progress;
use crate::story_relay::tick as tick_story_relay;
use crate::story_writes::tick as tick_story_writes;
use crate::teleport::{bring_guest_here, go_to_guest, tick as tick_teleport};

pub(super) fn run_queued_actions(build: &VerifiedBuild) {
    for action in take_all() {
        let outcome = match action {
            Action::ChangePhase { phase, point } => {
                change_phase(build, phase, point).map_err(|error| error.to_string())
            }
            Action::StoryJump(point) => run_story_jump(build, point),
            Action::BringGuestHere => bring_guest_here(build),
            Action::GoToGuest => go_to_guest(build),
            other => run_trainer_action(build, other).map_err(|error| error.to_string()),
        };
        match outcome {
            Ok(()) => log_line(&format!("action: {action:?}")),
            Err(error) => log_line(&format!("action: {action:?} failed ({error})")),
        }
    }
    tick_puppets(build);
    tick_ghost(build);
    tick_own_bullets(build);
    tick_own_powers(build);
    tick_partner_bullets(build);
    tick_enemy_bullets(build);
    tick_partner_powers(build);
    tick_effects(build);
    tick_music(build);
    tick_downed(build);
    tick_pickups(build);
    tick_stay_together(build);
    tick_dialogue(build);
    tick_fade_sync(build);
    tick_prologue_levels(build);
    tick_door_sync(build);
    tick_cutscene_skip(build);
    tick_fight_sync(build);
    tick_story_writes(build);
    tick_story_relay(build);
    tick_one_sided(build);
    tick_script_rolls(build);
    tick_boss_messages(build);
    tick_story_jump(build);
    tick_teleport(build);
    tick_story_progress(build);
    tick_containers(build);
}
