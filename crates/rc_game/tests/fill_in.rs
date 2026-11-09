use rc_game::fight_sync::SlotState;
use rc_game::script_functions::{
    FILL_IN_GRACE_MS, FILL_IN_QUEUED_CALL_POP_EXPECTED_MS, FUNCTIONS_CALLED_ONLY_FROM_LOCAL_STATE,
    FillInArrival, FillInCopy, FillInFlight, FillInNotice, FillInReach, FillInRelay,
    FunctionArrival, FunctionArrivalFacts, FunctionDrop, FunctionsRan, TracedRelay, fill_in_reach,
    function_arrival, script_function, traced_relay,
};

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

fn never_modal(_: &FillInCopy) -> bool {
    false
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
        assert_eq!(relay.tick(SlotState::Idle, now, never_modal), None);
    }
    let dropped = relay
        .ran_here(PROLOGUE_PHASE, PROLOGUE_START)
        .expect("the guest's own start replaces the host's copy");
    assert_eq!(dropped.arrived_ms, RUN34_HOST_COPY_ARRIVED_MS);
    let after_grace = RUN34_HOST_COPY_ARRIVED_MS + FILL_IN_GRACE_MS + 1_000;
    assert_eq!(relay.tick(SlotState::Idle, after_grace, never_modal), None);
    assert_eq!(relay.waiting_len(), 0);
}

#[test]
fn a_copy_this_game_never_runs_itself_fills_in_after_the_grace_time() {
    let mut relay = FillInRelay::new();
    arrive(&mut relay, PROLOGUE_PACK, PROLOGUE_PHASE, PROLOGUE_START, 0);
    assert_eq!(
        relay.tick(SlotState::Idle, FILL_IN_GRACE_MS - 1, never_modal),
        None
    );
    let copy = relay
        .tick(SlotState::Idle, FILL_IN_GRACE_MS, never_modal)
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
    assert_eq!(relay.tick(SlotState::Idle, 1_000, never_modal), None);
    let copy = relay
        .tick(SlotState::Idle, 1_000 + FILL_IN_GRACE_MS, never_modal)
        .unwrap();
    assert_eq!(copy.name, ENEMY_COUNT);
    relay.queued(ENEMY_COUNT, 4_000);
    assert!(relay.started(ENEMY_COUNT));
    assert_eq!(relay.tick(SlotState::Busy, 4_010, never_modal), None);
    assert_eq!(relay.tick(SlotState::Idle, 4_020, never_modal), None);
    assert_eq!(relay.in_flight(), &FillInFlight::Idle);
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
    let copy = relay.tick(SlotState::Idle, 500, never_modal).unwrap();
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
    assert_eq!(relay.tick(SlotState::Idle, 20, never_modal), None);
    let first = relay
        .tick(SlotState::Idle, FILL_IN_GRACE_MS, never_modal)
        .unwrap();
    assert_eq!(first.name, ENEMY_COUNT);
}

#[test]
fn only_one_copy_is_in_the_engine_queue_at_a_time() {
    let mut relay = FillInRelay::new();
    arrive(
        &mut relay,
        STONE_FIELD_PACK,
        STONE_FIELD_PHASE,
        BOOK_SCENE_END,
        0,
    );
    arrive(
        &mut relay,
        STONE_FIELD_PACK,
        STONE_FIELD_PHASE,
        BOOK_TALK_02,
        0,
    );
    let first = relay.tick(SlotState::Idle, 0, never_modal).unwrap();
    relay.queued(&first.name, 0);
    assert_eq!(
        relay.tick(SlotState::Idle, 10, never_modal),
        None,
        "queued, not popped"
    );
    assert!(!relay.started(BOOK_TALK_02));
    assert!(relay.started(BOOK_SCENE_END));
    assert_eq!(
        relay.tick(SlotState::Busy, 20, never_modal),
        None,
        "still running"
    );
    assert_eq!(
        relay.tick(SlotState::SteppingHeld, 30, never_modal),
        None,
        "held, not ended"
    );
    // The first idle tick ends the running copy, and the next one (also behind the hit counter)
    // goes on that tick.
    let second = relay.tick(SlotState::Idle, 40, never_modal).unwrap();
    assert_eq!(second.name, BOOK_TALK_02);
    assert_eq!(relay.in_flight(), &FillInFlight::Idle);
}

#[test]
fn a_queued_copy_that_never_starts_frees_the_relay_with_one_notice() {
    let mut relay = FillInRelay::new();
    arrive(
        &mut relay,
        STONE_FIELD_PACK,
        STONE_FIELD_PHASE,
        BOOK_SCENE_END,
        0,
    );
    let copy = relay.tick(SlotState::Idle, 0, never_modal).unwrap();
    relay.queued(&copy.name, 0);
    assert_eq!(
        relay.tick(
            SlotState::Idle,
            FILL_IN_QUEUED_CALL_POP_EXPECTED_MS,
            never_modal
        ),
        None
    );
    assert!(relay.take_notices().is_empty());
    relay.tick(
        SlotState::Idle,
        FILL_IN_QUEUED_CALL_POP_EXPECTED_MS + 1,
        never_modal,
    );
    assert_eq!(
        relay.take_notices(),
        [FillInNotice::QueuedCallNeverRan {
            name: BOOK_SCENE_END.to_owned(),
            waited_ms: FILL_IN_QUEUED_CALL_POP_EXPECTED_MS + 1
        }]
    );
    assert_eq!(relay.in_flight(), &FillInFlight::Idle);
}

#[test]
fn a_held_slot_waits_unless_the_copy_shows_modal_ui_which_closes_the_menu() {
    let mut relay = FillInRelay::new();
    arrive(
        &mut relay,
        STONE_FIELD_PACK,
        STONE_FIELD_PHASE,
        BOOK_SCENE_END,
        0,
    );
    assert_eq!(relay.tick(SlotState::SteppingHeld, 0, never_modal), None);
    assert_eq!(relay.tick(SlotState::Busy, 0, |_| true), None);
    let copy = relay.tick(SlotState::SteppingHeld, 0, |_| true).unwrap();
    assert_eq!(copy.name, BOOK_SCENE_END);
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
