use rc_game::marker::{UI_DIALOG_STATE_RVA, WorldFlags, read_world_flags};
use rc_game::stay_together::FollowRole;
use rc_game::story_jump::{
    JumpStep, PLAYER_UNIQUE_ACTOR_ID, SET_ITEM_NUM_NATIVE_RVA, SET_PLAYER_MP_FULL_MODE_NATIVE_RVA,
    SET_PLAYER_WEAPON_NATIVE_RVA, SET_TUTORIAL_NATIVE_RVA, SET_UNIQUE_ACTOR_LEVEL_NATIVE_RVA,
    StoryJumpView, check_story_jump, destination, jump_waits_for_pause_menu, loadout_steps,
};
use rc_game::{GameMemory, PhaseEntry, Replicant};
use rc_proto::{Message, StoryArray, StoryChange, StoryJump, StoryJumpPoint};

use crate::actions::change_phase;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net;
use crate::story_jump::natives::call_native;
use crate::story_jump::status::{PendingJump, remember_outcome};
use crate::story_progress::apply_change;

pub(crate) fn tick(build: &VerifiedBuild) {
    receive_from_host(build);
    run_pending_jump(build);
}

pub(crate) fn run_story_jump(build: &VerifiedBuild, point: StoryJumpPoint) -> Result<(), String> {
    let result = start_or_queue_jump(build, point);
    remember_outcome(match &result {
        Ok(()) => String::new(),
        Err(reason) => reason.clone(),
    });
    result
}

fn receive_from_host(build: &VerifiedBuild) {
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

fn read_view(build: &VerifiedBuild, host: bool) -> (StoryJumpView, WorldFlags) {
    let memory = InProcessMemory::new(build);
    let game = Replicant::new(InProcessMemory::new(build));
    let flags = read_world_flags(&memory).unwrap_or(WorldFlags {
        event_scene: false,
        control_mask: 0,
        main_menu_open: false,
        talk_window_open: false,
    });
    let view = StoryJumpView {
        host,
        map_manager_active: game.map_manager_is_active().unwrap_or(false),
        event_scene: flags.event_scene,
        talk_window_open: flags.talk_window_open,
        ui_dialog_state: memory
            .read_i32(build.module_base() + u64::from(UI_DIALOG_STATE_RVA))
            .unwrap_or(0),
        pause_menu_open: flags.main_menu_open,
    };
    (view, flags)
}

fn local_is_host() -> Result<bool, String> {
    let net = net::handle().ok_or("not in a co-op session")?;
    Ok(FollowRole::from_slot(net.view().local_slot) == FollowRole::Host)
}

fn refusal_text(
    refusal: &impl std::fmt::Display,
    view: &StoryJumpView,
    flags: &WorldFlags,
) -> String {
    format!(
        "{refusal} (event scene {}, talk {}, dialog state {:#x}, control mask {:#x})",
        view.event_scene, view.talk_window_open, view.ui_dialog_state, flags.control_mask
    )
}

fn start_or_queue_jump(build: &VerifiedBuild, point: StoryJumpPoint) -> Result<(), String> {
    let (view, flags) = read_view(build, local_is_host()?);
    check_story_jump(&view).map_err(|refusal| refusal_text(&refusal, &view, &flags))?;
    if jump_waits_for_pause_menu(&view) {
        PendingJump::set(point);
        log_line("story jump: waiting for the menu to close");
        return Ok(());
    }
    jump_as_host(build, point)
}

fn run_pending_jump(build: &VerifiedBuild) {
    let Some(point) = PendingJump::get() else {
        return;
    };
    let host = local_is_host().unwrap_or(false);
    let (view, flags) = read_view(build, host);
    if let Err(refusal) = check_story_jump(&view) {
        PendingJump::clear();
        let reason = refusal_text(&refusal, &view, &flags);
        log_line(&format!("story jump: dropped while waiting ({reason})"));
        remember_outcome(reason);
        return;
    }
    if jump_waits_for_pause_menu(&view) {
        return;
    }
    PendingJump::clear();
    log_line("story jump: running");
    let result = jump_as_host(build, point);
    if let Err(reason) = &result {
        log_line(&format!("story jump: failed ({reason})"));
    }
    remember_outcome(result.err().unwrap_or_default());
}

fn jump_as_host(build: &VerifiedBuild, point: StoryJumpPoint) -> Result<(), String> {
    let net = net::handle().ok_or("not in a co-op session")?;
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
