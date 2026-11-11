use std::sync::{Mutex, PoisonError};

use replicant_game::Replicant;
use replicant_game::readiness::{classify, read_play_checks};
use replicant_game::side_view::{
    DoorMoveFacts, IndoorArrivalReading, IndoorArrivalWatch, door_steps_for_a_move,
};
use replicant_game::stay_together::local_phase_index;
use replicant_proto::StoryCall;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::own_side_view_door;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::stay_together::{game_says_free_to_act, side_view_now};
use crate::story_relay::{
    RelayPath, relay_calls_are_clear, relay_queue_has_room, submit_relay_call,
};

static INDOOR_ARRIVAL: Mutex<IndoorArrivalWatch> = Mutex::new(IndoorArrivalWatch::new());

pub(crate) fn door_steps_for_a_move_here(
    build: &VerifiedBuild,
    target_in_side_view: bool,
    target_door: u16,
) -> &'static [&'static str] {
    let game = Replicant::new(InProcessMemory::new(build));
    let Ok(area) = game.current_phase() else {
        return &[];
    };
    door_steps_for_a_move(&DoorMoveFacts {
        area: &area,
        moved_in_side_view: side_view_now(build),
        moved_door: own_side_view_door(),
        target_in_side_view,
        target_door,
    })
}

pub(crate) fn door_steps_can_run() -> bool {
    relay_queue_has_room()
}

pub(crate) fn door_steps_are_running() -> bool {
    !relay_calls_are_clear()
}

pub(crate) fn run_door_steps(build: &VerifiedBuild, steps: &[&str], why: &str) {
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
    let door = INDOOR_ARRIVAL
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
    let Some(door) = door else {
        return;
    };
    if !door_steps_can_run() {
        log_line(
            "side view: you arrived inside a door's place without its view, and the call queue is full",
        );
        return;
    }
    run_door_steps(
        build,
        door.entry_steps,
        "the arrival put you inside a door's place without its view",
    );
}
