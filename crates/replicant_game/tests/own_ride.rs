use replicant_game::message_handlers::{MessageHandlers, is_player_local_handler};
use replicant_game::own_ride::{
    RIDE_END_MOVES_THE_PLAYER_FOR_MS, RideChange, RideWatch, is_a_ride_function, is_the_riders_own,
    leads_into_the_ride,
};
use replicant_game::script_functions::{
    FunctionArrival, FunctionArrivalFacts, FunctionDrop, FunctionsRan, function_arrival,
};
use replicant_game::stay_together::{
    FollowRole, JumpFacts, JumpPartner, JumpStep, SCRIPT_JUMP_SETTLE_MS, ScriptJumpWatch,
};
use replicant_game::story_calls::{SceneScope, StoryCallKind};
use replicant_game::story_relay::{
    CallKey, DropReason, FireAction, FireFacts, RunAction, RunFacts, StoryRelayLedger,
    TriggerAction,
};

const TOWN: &str = "A_DESERT_TOWN_01";
const TOWN_PHASE: u16 = 7;
const SECOND_RIDE_STEP: &str = "EID_1410_a0100g_21_04";
const FIRST_RIDE_STEP: &str = "EID_1410_a0100g_05_01";
const RIDE_START: &str = "Put_Boat_02";
const RIDE_ARRIVAL: &str = "Put_Boat_03";
const RIDE_SKIP: &str = "Put_Boat_04";
const RIDE_ARRIVAL_MESSAGE: i32 = 217;
const HOST_RIDE_STARTED_MS: u64 = 13_011;
const HOST_SKIPPED_MS: u64 = 20_086;
const HOST_MOVED_TO_THE_STOP_MS: u64 = 22_318;
const HOST_BEFORE_THE_STOP: [f32; 3] = [-41.5, 14.0, 11.6];
const HOST_AT_THE_STOP: [f32; 3] = [49.4, 14.4, 24.2];
const GUEST_FAR_FROM_THE_STOP: JumpPartner = JumpPartner {
    position: [-133.6, 14.0, 17.7],
    phase: TOWN_PHASE,
};
const SLOT: u64 = 0x1000;
const KEY: CallKey = CallKey {
    function: 0x850e_cd75_aabb_9def,
    args: 0xcbf2_9ce4_8422_2325,
};

fn fire_facts(role: FollowRole, function: &str) -> FireFacts {
    FireFacts {
        enabled: true,
        connected: true,
        role,
        local_in_play: true,
        partner_in_same_phase: true,
        kind: Some(StoryCallKind::Story),
        scope: SceneScope::of(function),
        partner_here: true,
        owned_elsewhere: false,
        args_relayable: true,
        partner_too_far: false,
        can_gather_partner: false,
        each_run_counts: false,
        partners_copy: false,
    }
}

fn run_facts(role: FollowRole, function: &str) -> RunFacts {
    RunFacts {
        enabled: true,
        role,
        local_in_play: true,
        local_phase: Some(TOWN_PHASE),
        call_phase: TOWN_PHASE,
        known_story_call: true,
        scope: SceneScope::of(function),
        partner_here: true,
        each_run_counts: false,
    }
}

fn arrival_facts() -> FunctionArrivalFacts<'static> {
    FunctionArrivalFacts {
        enabled: true,
        local_in_play: true,
        local_phase: Some(TOWN_PHASE),
        call_phase: TOWN_PHASE,
        phase_label: TOWN,
        queue_is_empty: true,
    }
}

fn jump(now_ms: u64, position: [f32; 3], own_ride_moves_the_host: bool) -> JumpFacts {
    JumpFacts {
        now_ms,
        position: Some(position),
        phase: Some(TOWN_PHASE),
        script_pause_in_force: true,
        mod_moved_the_host: false,
        own_ride_moves_the_host,
        partner: Some(GUEST_FAR_FROM_THE_STOP),
    }
}

#[test]
fn the_ride_and_every_step_that_boards_it_are_the_riders_own() {
    for name in [
        RIDE_START,
        RIDE_ARRIVAL,
        RIDE_SKIP,
        "Put_Boat_05",
        "Put_Boat_06",
    ] {
        assert!(is_a_ride_function(name), "{name}");
        assert!(is_the_riders_own(name), "{name}");
        assert_eq!(SceneScope::of(name), SceneScope::LocalScene, "{name}");
    }
    for name in [FIRST_RIDE_STEP, SECOND_RIDE_STEP, "EID_1410_a0100g_06"] {
        assert!(leads_into_the_ride(name), "{name}");
        assert_eq!(SceneScope::of(name), SceneScope::LocalScene, "{name}");
    }
}

#[test]
fn the_tour_and_its_skip_answers_are_not_the_riders_own() {
    for name in [
        "EID_1410_a0100g_Start",
        "EID_1410_a0100g_15_01",
        "EID_1410_a0100g_16_GuideSkip_01",
        "EID_1410_a0100g_16_GuideSkip_02",
        "EID_1410_a0100g_21_05",
        "Put_Boat_Sendoh_01",
    ] {
        assert!(!is_the_riders_own(name), "{name}");
    }
}

#[test]
fn the_hosts_step_that_boards_runs_there_only_and_is_never_sent() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_fire(
            SLOT,
            &fire_facts(FollowRole::Host, SECOND_RIDE_STEP),
            KEY,
            0
        ),
        FireAction::RunLocalSceneOnly
    );
    assert_eq!(
        ledger.on_fire(
            SLOT,
            &fire_facts(FollowRole::Guest, SECOND_RIDE_STEP),
            KEY,
            0
        ),
        FireAction::RunLocalSceneOnly
    );
}

#[test]
fn a_partners_run_or_trigger_of_a_step_that_boards_is_never_run_here() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_run(&run_facts(FollowRole::Guest, SECOND_RIDE_STEP), KEY, 4_751),
        RunAction::Drop(DropReason::LocalScene)
    );
    assert_eq!(
        ledger.on_trigger(&run_facts(FollowRole::Host, SECOND_RIDE_STEP), KEY, 4_751),
        TriggerAction::Drop(DropReason::LocalScene)
    );
}

#[test]
fn the_rides_arrival_is_never_a_host_driven_handler() {
    let mut handlers = MessageHandlers::new();
    handlers.register(TOWN_PHASE, RIDE_ARRIVAL, RIDE_ARRIVAL_MESSAGE);
    assert!(is_player_local_handler(RIDE_ARRIVAL, RIDE_ARRIVAL_MESSAGE));
    assert!(!handlers.is_host_driven(TOWN_PHASE, RIDE_ARRIVAL));
}

#[test]
fn a_partners_shared_copy_of_a_ride_step_or_function_is_dropped_on_arrival() {
    let ran = FunctionsRan::new();
    for name in [SECOND_RIDE_STEP, FIRST_RIDE_STEP, RIDE_ARRIVAL] {
        assert_eq!(
            function_arrival(&arrival_facts(), name, &ran, &[]),
            FunctionArrival::Drop(FunctionDrop::TheRidersOwn),
            "{name}"
        );
    }
    assert_eq!(
        function_arrival(
            &arrival_facts(),
            "EID_1410_a0100g_16_GuideSkip_01",
            &ran,
            &[]
        ),
        FunctionArrival::Drop(FunctionDrop::CarriedByTheSkipVote)
    );
}

#[test]
fn the_ride_is_watched_from_its_start_to_its_skip() {
    let mut watch = RideWatch::new();
    watch.enter_phase(Some(TOWN_PHASE));
    assert!(!watch.moves_the_player(0));
    assert_eq!(
        watch.ran(RIDE_START, HOST_RIDE_STARTED_MS),
        Some(RideChange::Started)
    );
    assert_eq!(watch.ran(RIDE_START, HOST_RIDE_STARTED_MS + 16), None);
    assert!(watch.is_riding());
    assert_eq!(watch.ran(SECOND_RIDE_STEP, 15_000), None);
    assert_eq!(
        watch.ran(RIDE_SKIP, HOST_SKIPPED_MS),
        Some(RideChange::Ended {
            by: RIDE_SKIP,
            ridden_ms: Some(HOST_SKIPPED_MS - HOST_RIDE_STARTED_MS),
        })
    );
    assert!(!watch.is_riding());
    assert!(watch.moves_the_player(HOST_MOVED_TO_THE_STOP_MS));
    assert!(!watch.moves_the_player(HOST_SKIPPED_MS + RIDE_END_MOVES_THE_PLAYER_FOR_MS));
}

#[test]
fn an_end_without_a_seen_start_still_counts_as_the_rides_move() {
    let mut watch = RideWatch::new();
    assert_eq!(
        watch.ran(RIDE_ARRIVAL, 1_000),
        Some(RideChange::Ended {
            by: RIDE_ARRIVAL,
            ridden_ms: None,
        })
    );
    assert!(watch.moves_the_player(1_000));
}

#[test]
fn a_new_place_forgets_the_ride() {
    let mut watch = RideWatch::new();
    watch.enter_phase(Some(TOWN_PHASE));
    watch.ran(RIDE_START, 0);
    watch.enter_phase(Some(TOWN_PHASE + 1));
    assert!(!watch.is_riding());
    assert!(!watch.moves_the_player(1));
}

#[test]
fn the_hosts_ride_end_at_the_stop_never_brings_the_guest() {
    let mut watch = ScriptJumpWatch::new();
    watch.observe(&jump(HOST_SKIPPED_MS, HOST_BEFORE_THE_STOP, true));
    watch.observe(&jump(HOST_MOVED_TO_THE_STOP_MS, HOST_AT_THE_STOP, true));
    let mut settled = jump(
        HOST_MOVED_TO_THE_STOP_MS + SCRIPT_JUMP_SETTLE_MS,
        HOST_AT_THE_STOP,
        true,
    );
    settled.script_pause_in_force = false;
    assert_eq!(watch.observe(&settled), JumpStep::Nothing);
}

#[test]
fn the_same_move_without_a_ride_still_brings_the_guest() {
    let mut watch = ScriptJumpWatch::new();
    watch.observe(&jump(HOST_SKIPPED_MS, HOST_BEFORE_THE_STOP, false));
    watch.observe(&jump(HOST_MOVED_TO_THE_STOP_MS, HOST_AT_THE_STOP, false));
    let mut settled = jump(
        HOST_MOVED_TO_THE_STOP_MS + SCRIPT_JUMP_SETTLE_MS,
        HOST_AT_THE_STOP,
        false,
    );
    settled.script_pause_in_force = false;
    assert!(matches!(
        watch.observe(&settled),
        JumpStep::BringPartner { .. }
    ));
}
