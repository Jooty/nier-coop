use replicant_game::fight_sync::SlotState;
use replicant_game::script_functions::{
    FILL_IN_GRACE_MS, FUNCTIONS_CALLED_ONLY_FROM_LOCAL_STATE, FillInArrival, FillInReach,
    FillInRelay, FunctionArrival, FunctionArrivalFacts, FunctionDrop, FunctionsRan, TracedRelay,
    fill_in_reach, function_arrival, script_function, traced_relay,
};
use replicant_game::story_relay::RelayCallGate;
use replicant_proto::StoryCall;

const PROLOGUE_PACK: &str = "A_TOKYO_FIELD_01";
const PROLOGUE_PHASE: u16 = 25;
const PROLOGUE_START: &str = "EID_0050_a0010_Start";
const PROLOGUE_END: &str = "EID_0050_a0010_End";
const STONE_FIELD_PACK: &str = "A_STONE_FIELD_02";
const STONE_FIELD_PHASE: u16 = 23;
const ENEMY_COUNT: &str = "EID_0200_a0037g_EnemyCount";
const WAVE_END: &str = "EID_0200_a0037g_02_End";
const BOOK_SCENE_END: &str = "EID_0200_a0030g_End";
const BOOK_TALK_02: &str = "EID_0200_a0030g_Talk02";
const RUN34_HOST_COPY_ARRIVED_MS: u64 = 5_664;
const RUN34_GUEST_OWN_START_TRACED_MS: u64 = 7_232;

fn facts(label: &'static str, phase: u16) -> FunctionArrivalFacts<'static> {
    FunctionArrivalFacts {
        enabled: true,
        local_in_play: true,
        local_phase: Some(phase),
        call_phase: phase,
        phase_label: label,
        queue_is_empty: true,
    }
}

fn never_modal(_: &StoryCall) -> bool {
    false
}

fn call(phase: u16, name: &str) -> StoryCall {
    StoryCall {
        phase,
        pack: 0,
        function: name.to_owned(),
        args: Vec::new(),
    }
}

fn arrive(relay: &mut FillInRelay, label: &str, phase: u16, name: &str, now_ms: u64) {
    let reach = fill_in_reach(label, name);
    assert!(matches!(
        relay.arrive(phase, name, reach, now_ms),
        FillInArrival::Waiting { .. }
    ));
}

#[test]
fn run34_the_hosts_prologue_start_waits_for_the_guests_own_start_and_is_dropped_when_it_runs() {
    // Before: the host's copy of a scene start forced a drain and ran at once (run34 G 241-245),
    // before the guest's own game had placed its player.
    let verdict = function_arrival(
        &facts(PROLOGUE_PACK, PROLOGUE_PHASE),
        PROLOGUE_START,
        &FunctionsRan::new(),
        &[],
    );
    assert_eq!(verdict, FunctionArrival::ForcedDrain);
    assert_eq!(
        fill_in_reach(PROLOGUE_PACK, PROLOGUE_START),
        FillInReach::OwnCopyExpected,
        "StartStage calls it on each game"
    );
    let mut relay = FillInRelay::new();
    assert_eq!(
        relay.arrive(
            PROLOGUE_PHASE,
            PROLOGUE_START,
            FillInReach::OwnCopyExpected,
            RUN34_HOST_COPY_ARRIVED_MS
        ),
        FillInArrival::Waiting {
            position: 1,
            due_in_ms: FILL_IN_GRACE_MS
        }
    );
    for now in (RUN34_HOST_COPY_ARRIVED_MS..RUN34_GUEST_OWN_START_TRACED_MS).step_by(16) {
        assert_eq!(relay.tick(true, now), None);
    }
    let dropped = relay
        .ran_here(PROLOGUE_PHASE, PROLOGUE_START)
        .expect("the guest's own start replaces the host's copy");
    assert_eq!(dropped.arrived_ms, RUN34_HOST_COPY_ARRIVED_MS);
    let after_grace = RUN34_HOST_COPY_ARRIVED_MS + FILL_IN_GRACE_MS + 1_000;
    assert_eq!(relay.tick(true, after_grace), None);
    assert_eq!(relay.waiting_len(), 0);
}

#[test]
fn a_copy_this_game_never_runs_itself_fills_in_after_the_grace_time() {
    let mut relay = FillInRelay::new();
    arrive(&mut relay, PROLOGUE_PACK, PROLOGUE_PHASE, PROLOGUE_START, 0);
    assert_eq!(relay.tick(true, FILL_IN_GRACE_MS - 1), None);
    let copy = relay
        .tick(true, FILL_IN_GRACE_MS)
        .expect("due after the grace time");
    assert_eq!(copy.name, PROLOGUE_START);
}

#[test]
fn run34_enemy_count_is_relayed_when_popped_and_fills_in_on_the_guest() {
    // The host's engine popped it (group 6 emptied); it is not in the shared function table.
    assert!(script_function(STONE_FIELD_PACK, ENEMY_COUNT).is_none());
    assert_eq!(
        traced_relay(STONE_FIELD_PACK, ENEMY_COUNT, true, false),
        TracedRelay::Send
    );
    assert_eq!(
        traced_relay(STONE_FIELD_PACK, ENEMY_COUNT, false, false),
        TracedRelay::NotRelayed,
        "only a popped event callback outside the table is relayed"
    );
    // The guest accepts it as a plain step.
    let verdict = function_arrival(
        &facts(STONE_FIELD_PACK, STONE_FIELD_PHASE),
        ENEMY_COUNT,
        &FunctionsRan::new(),
        &[],
    );
    assert_eq!(verdict, FunctionArrival::RunNow);
    // The guest's group 6 never empties on its own (the host kills the copies), so after the
    // grace time the host's copy runs here once and sets count_kill_enemy before the wave end.
    let mut relay = FillInRelay::new();
    arrive(
        &mut relay,
        STONE_FIELD_PACK,
        STONE_FIELD_PHASE,
        ENEMY_COUNT,
        1_000,
    );
    assert_eq!(relay.tick(true, 1_000), None);
    let copy = relay.tick(true, 1_000 + FILL_IN_GRACE_MS).unwrap();
    assert_eq!(copy.name, ENEMY_COUNT);
    let mut gate = RelayCallGate::new();
    gate.submit(call(STONE_FIELD_PHASE, &copy.name), (), 4_000);
    assert!(gate.tick(SlotState::Idle, false, never_modal).is_some());
    gate.queued(ENEMY_COUNT, 0x48);
    assert!(gate.popped(0x48));
    assert_eq!(gate.tick(SlotState::Busy, false, never_modal), None);
    assert!(!gate.is_clear());
    assert_eq!(gate.tick(SlotState::Idle, false, never_modal), None);
    assert!(gate.is_clear());
    assert_eq!(
        relay.arrive(
            STONE_FIELD_PHASE,
            ENEMY_COUNT,
            FillInReach::OwnCopyExpected,
            4_030
        ),
        FillInArrival::Waiting {
            position: 1,
            due_in_ms: FILL_IN_GRACE_MS
        },
        "a later wave's count is a new copy"
    );
}

#[test]
fn popped_ends_gated_callbacks_and_message_handlers_stay_on_the_callback_path() {
    assert_eq!(
        traced_relay(STONE_FIELD_PACK, WAVE_END, true, false),
        TracedRelay::CallbackPathRelays
    );
    assert_eq!(
        traced_relay(PROLOGUE_PACK, PROLOGUE_END, true, false),
        TracedRelay::CallbackPathRelays
    );
    assert_eq!(
        traced_relay(STONE_FIELD_PACK, BOOK_TALK_02, true, true),
        TracedRelay::CallbackPathRelays
    );
    assert_eq!(
        traced_relay(STONE_FIELD_PACK, BOOK_TALK_02, false, false),
        TracedRelay::Send
    );
    assert_eq!(
        traced_relay(STONE_FIELD_PACK, "AttackCount_Talk", true, false),
        TracedRelay::NotRelayed
    );
}

#[test]
fn a_step_behind_a_local_counter_runs_at_once() {
    assert_eq!(
        fill_in_reach(STONE_FIELD_PACK, BOOK_SCENE_END),
        FillInReach::CannotReachHere,
        "AttackCount_Talk and Life_Judgment call it behind their counters"
    );
    let mut relay = FillInRelay::new();
    arrive(
        &mut relay,
        STONE_FIELD_PACK,
        STONE_FIELD_PHASE,
        BOOK_SCENE_END,
        500,
    );
    let copy = relay.tick(true, 500).unwrap();
    assert_eq!(copy.name, BOOK_SCENE_END);
}

#[test]
fn copies_keep_the_partners_order_and_a_later_copy_never_overtakes_a_waiting_one() {
    let mut relay = FillInRelay::new();
    arrive(
        &mut relay,
        STONE_FIELD_PACK,
        STONE_FIELD_PHASE,
        ENEMY_COUNT,
        0,
    );
    arrive(
        &mut relay,
        STONE_FIELD_PACK,
        STONE_FIELD_PHASE,
        BOOK_SCENE_END,
        10,
    );
    assert_eq!(relay.tick(true, 20), None);
    let first = relay.tick(true, FILL_IN_GRACE_MS).unwrap();
    assert_eq!(first.name, ENEMY_COUNT);
}

#[test]
fn a_copy_waits_while_the_relay_call_gate_is_busy() {
    let mut relay = FillInRelay::new();
    arrive(
        &mut relay,
        STONE_FIELD_PACK,
        STONE_FIELD_PHASE,
        BOOK_SCENE_END,
        0,
    );
    assert_eq!(
        relay.tick(false, 10),
        None,
        "another partner call is in flight"
    );
    assert_eq!(relay.tick(true, 20).unwrap().name, BOOK_SCENE_END);
}

#[test]
fn the_same_copy_waits_once_and_a_phase_change_drops_the_old_phases_copies() {
    let mut relay = FillInRelay::new();
    arrive(&mut relay, PROLOGUE_PACK, PROLOGUE_PHASE, PROLOGUE_START, 0);
    assert_eq!(
        relay.arrive(
            PROLOGUE_PHASE,
            PROLOGUE_START,
            FillInReach::OwnCopyExpected,
            5
        ),
        FillInArrival::AlreadyWaiting
    );
    arrive(
        &mut relay,
        STONE_FIELD_PACK,
        STONE_FIELD_PHASE,
        ENEMY_COUNT,
        5,
    );
    assert_eq!(relay.enter_phase(Some(STONE_FIELD_PHASE)), 1);
    assert_eq!(relay.waiting_len(), 1);
    assert_eq!(relay.ran_here(PROLOGUE_PHASE, PROLOGUE_START), None);
    assert_eq!(relay.clear(), 1);
}

#[test]
fn an_event_callback_outside_the_table_that_ran_here_is_dropped() {
    let mut ran = FunctionsRan::new();
    ran.note(STONE_FIELD_PHASE, ENEMY_COUNT);
    assert_eq!(
        function_arrival(
            &facts(STONE_FIELD_PACK, STONE_FIELD_PHASE),
            ENEMY_COUNT,
            &ran,
            &[]
        ),
        FunctionArrival::Drop(FunctionDrop::AlreadyRan)
    );
}

#[test]
fn the_unreachable_table_is_sorted_and_names_shared_functions_only() {
    assert!(
        FUNCTIONS_CALLED_ONLY_FROM_LOCAL_STATE
            .windows(2)
            .all(|pair| pair[0] < pair[1])
    );
    for (pack, name) in FUNCTIONS_CALLED_ONLY_FROM_LOCAL_STATE {
        assert!(script_function(pack, name).is_some(), "{pack} {name}");
    }
}
