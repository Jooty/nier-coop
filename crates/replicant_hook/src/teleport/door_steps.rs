use std::sync::{Mutex, PoisonError};

use replicant_game::Replicant;
use replicant_game::readiness::{classify, read_play_checks};
use replicant_game::side_view::{
    ArrivalViewFix, DoorMoveAction, DoorMoveFacts, IndoorArrivalReading, IndoorArrivalWatch,
    OPEN_PLAYER_GAUGES_SCRIPT_FUNCTION, SideViewDoor, ViewOffArg, door_action_for_a_move,
    view_off_calls,
};
use replicant_game::stay_together::local_phase_index;
use replicant_proto::StoryCall;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::own_side_view_door;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::stay_together::{game_says_free_to_act, side_view_now};
use crate::story_jump::{NativeArg, call_native_with};
use crate::story_relay::{
    RelayPath, relay_calls_are_clear, relay_queue_has_room, submit_relay_call,
};

static INDOOR_ARRIVAL: Mutex<IndoorArrivalWatch> = Mutex::new(IndoorArrivalWatch::new());

pub(crate) fn door_action_for_a_move_here(
    build: &VerifiedBuild,
    target_in_side_view: bool,
    target_door: u16,
) -> DoorMoveAction {
    let game = Replicant::new(InProcessMemory::new(build));
    let Ok(area) = game.current_phase() else {
        return DoorMoveAction::Nothing;
    };
    door_action_for_a_move(&DoorMoveFacts {
        area: &area,
        moved_in_side_view: side_view_now(build),
        moved_door: own_side_view_door(),
        target_in_side_view,
        target_door,
    })
}

pub(crate) fn run_door_action(build: &VerifiedBuild, action: DoorMoveAction, why: &str) {
    match action {
        DoorMoveAction::Nothing => {}
        DoorMoveAction::RunSteps(steps) => run_door_steps(build, steps, why),
        DoorMoveAction::ViewOffRoutine(door) => run_view_off_routine(build, Some(door), why),
    }
}

pub(crate) fn note_mod_area_change(build: &VerifiedBuild) {
    INDOOR_ARRIVAL
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .mod_requested_an_area_change(side_view_now(build));
}

fn run_view_off_routine(build: &VerifiedBuild, left_door: Option<&SideViewDoor>, why: &str) {
    log_line(&format!(
        "side view: {why}, turning the view off with the game's own calls"
    ));
    for call in view_off_calls(left_door) {
        let arguments: Vec<NativeArg> = call
            .args
            .iter()
            .map(|arg| match *arg {
                ViewOffArg::Int(value) => NativeArg::Integer(value),
                ViewOffArg::Real(value) => NativeArg::Real(value),
                ViewOffArg::Text(text) => NativeArg::Text(text),
            })
            .collect();
        call_native_with(build, call.native_rva, &arguments);
    }
    run_door_steps(build, &[OPEN_PLAYER_GAUGES_SCRIPT_FUNCTION], why);
}

pub(crate) fn door_steps_can_run() -> bool {
    relay_queue_has_room()
}

pub(crate) fn door_steps_are_running() -> bool {
    !relay_calls_are_clear()
}

fn run_door_steps(build: &VerifiedBuild, steps: &[&str], why: &str) {
    let game = Replicant::new(InProcessMemory::new(build));
    let Some(phase) = local_phase_index(&game) else {
        return;
    };
    for step in steps {
        let call = StoryCall {
            phase,
            pack: 0,
            function: (*step).to_owned(),
            args: Vec::new(),
        };
        let queued = submit_relay_call(&call, RelayPath::HeldOwnCall);
        log_line(&format!(
            "side view: {why}, running the door's own {step} here (queued {queued})"
        ));
    }
}

pub(super) fn tick_indoor_arrival(build: &VerifiedBuild) {
    let game = Replicant::new(InProcessMemory::new(build));
    let area = game.current_phase().ok();
    let fix = INDOOR_ARRIVAL
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(&IndoorArrivalReading {
            now_ms: net_clock_ms(),
            area_loads: crate::player_body::phase_load_count(),
            area: area.as_deref(),
            start_point: game.arrival_point().unwrap_or(0),
            in_play_and_free: classify(read_play_checks(&game)).is_in_play()
                && game_says_free_to_act(build).unwrap_or(false),
            in_side_view: side_view_now(build),
        });
    match fix {
        ArrivalViewFix::Nothing => {}
        ArrivalViewFix::RunEntrySteps(door) if door_steps_can_run() => run_door_steps(
            build,
            door.entry_steps,
            "the arrival put you inside a door's place without its view",
        ),
        ArrivalViewFix::RunEntrySteps(_) => log_line(
            "side view: you arrived inside a door's place without its view, and the call queue is full",
        ),
        ArrivalViewFix::TurnViewOff => run_view_off_routine(
            build,
            None,
            "the mod's area change carried a door place's view into this area",
        ),
    }
}
