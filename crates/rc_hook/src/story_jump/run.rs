use rc_game::PhaseEntry;
use rc_game::Replicant;
use rc_game::stay_together::FollowRole;
use rc_game::story_jump::{
    JumpStep, PLAYER_UNIQUE_ACTOR_ID, SET_ITEM_NUM_NATIVE_RVA, SET_PLAYER_MP_FULL_MODE_NATIVE_RVA,
    SET_PLAYER_WEAPON_NATIVE_RVA, SET_TUTORIAL_NATIVE_RVA, SET_UNIQUE_ACTOR_LEVEL_NATIVE_RVA,
    StoryJumpView, check_story_jump, destination, loadout_steps,
};
use rc_proto::{Message, StoryArray, StoryChange, StoryJump, StoryJumpPoint};

use crate::actions::change_phase;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net;
use crate::stay_together::game_says_free_to_act;
use crate::story_jump::natives::call_native;
use crate::story_jump::status::remember_outcome;
use crate::story_progress::apply_change;

pub(crate) fn run_story_jump(build: &VerifiedBuild, point: StoryJumpPoint) -> Result<(), String> {
    let result = jump_as_host(build, point);
    remember_outcome(match &result {
        Ok(()) => String::new(),
        Err(reason) => reason.clone(),
    });
    result
}

pub(crate) fn receive_from_host(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    for message in net.take_relayed(|message| matches!(message, Message::StoryJump(_))) {
        let Message::StoryJump(jump) = message else {
            continue;
        };
        log_line(&format!("story jump: the host jumped ({:?})", jump.point));
        apply_loadout(build, jump.point);
        log_line("story jump: loadout applied, following the host");
    }
}

fn jump_as_host(build: &VerifiedBuild, point: StoryJumpPoint) -> Result<(), String> {
    let net = net::handle().ok_or("not in a co-op session")?;
    let game = Replicant::new(InProcessMemory::new(build));
    let view = StoryJumpView {
        host: FollowRole::from_slot(net.view().local_slot) == FollowRole::Host,
        map_manager_active: game.map_manager_is_active().unwrap_or(false),
        free_to_act: game_says_free_to_act(build).unwrap_or(false),
    };
    check_story_jump(&view).map_err(|refusal| refusal.to_string())?;
    let (phase_name, phase_point) = destination(point);
    let phase = PhaseEntry::find(phase_name).ok_or("destination phase missing")?;
    net.send_relayed(Message::StoryJump(StoryJump { point }));
    apply_loadout(build, point);
    change_phase(build, phase, phase_point).map_err(|error| error.to_string())?;
    log_line(&format!(
        "story jump: changing phase to {phase_name} point {phase_point}"
    ));
    Ok(())
}

fn apply_loadout(build: &VerifiedBuild, point: StoryJumpPoint) {
    for step in loadout_steps(point) {
        apply_step(build, step);
        log_line(&format!("story jump: {step:?}"));
    }
}

fn apply_step(build: &VerifiedBuild, step: JumpStep) {
    match step {
        JumpStep::SetFlag { id } => apply_change(build, game_flag(id, true)),
        JumpStep::ClearFlag { id } => apply_change(build, game_flag(id, false)),
        JumpStep::SetTutorial { id } => {
            call_native(build, SET_TUTORIAL_NATIVE_RVA, &[i32::from(id)]);
        }
        JumpStep::SetItem { id, count } => {
            call_native(build, SET_ITEM_NUM_NATIVE_RVA, &[id, count]);
        }
        JumpStep::EquipWeapon { id } => call_native(build, SET_PLAYER_WEAPON_NATIVE_RVA, &[id]),
        JumpStep::SetPlayerLevel { level } => call_native(
            build,
            SET_UNIQUE_ACTOR_LEVEL_NATIVE_RVA,
            &[PLAYER_UNIQUE_ACTOR_ID, level],
        ),
        JumpStep::MpFullMode { on } => {
            call_native(build, SET_PLAYER_MP_FULL_MODE_NATIVE_RVA, &[i32::from(on)]);
        }
    }
}

fn game_flag(id: u16, on: bool) -> StoryChange {
    StoryChange::Flag {
        array: StoryArray::GameFlags,
        id,
        on,
    }
}
