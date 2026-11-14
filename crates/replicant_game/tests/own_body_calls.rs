use replicant_game::stay_together::FollowRole;
use replicant_game::story_calls::{
    SceneScope, StoryCallKind, is_own_body_callback_name, story_call_kind_by_name,
};
use replicant_game::story_relay::{
    CallKey, DropReason, FireAction, FireFacts, RunAction, RunFacts, StoryRelayLedger,
    TriggerAction,
};
use replicant_game::story_writes::{is_scene_write_for_the_host, watches_scene_writes};
use replicant_proto::{StoryArray, StoryChange};

const DROWN_PACK: &str = "A_DESERT_TOWN_01";
const DROWN_CALLBACK: &str = "PlayerDrawnCallback";
const DROWN_KEY: CallKey = CallKey {
    function: 0xD559_D08E_B346_E5AF,
    args: 0x5E1F_6A25_C995_0184,
};
const DROWN_ARGUMENT_POINTER: u64 = 0x1000;
const DROWN_COUNT_SNOW_FLAG: u16 = 1041;
const OWN_FALL_AND_RESPAWN_CALLBACKS: [&str; 12] = [
    "PlayerDrawnCallback",
    "Drown_Restart",
    "Drown_Restart_02",
    "Drown_Restart_End",
    "Fall_ReStart",
    "Fall_ReStart01",
    "Fall_ReStart02",
    "Fall_ReStart03",
    "Fall_ReStart_01",
    "Fall_ReStart_04",
    "Fall_Warp",
    "SIDEVIEW_FALL01",
];
const FIGHT_FIELD_FALL_TRAPS_LEFT_AS_THEY_WERE: [&str; 3] =
    ["FALL_TRAP_01_04", "FALL_TRAP_01_04_01", "FALL_TRAP_05"];

fn fire_facts(role: FollowRole, function: &str) -> FireFacts {
    FireFacts {
        enabled: true,
        connected: true,
        role,
        local_in_play: true,
        partner_in_same_phase: true,
        kind: Some(story_call_kind_by_name(DROWN_PACK, function)),
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
        local_phase: Some(7),
        call_phase: 7,
        known_story_call: true,
        scope: SceneScope::of(function),
        partner_here: true,
        each_run_counts: false,
    }
}

#[test]
fn the_drowning_callback_is_a_story_call_in_its_pack() {
    assert_eq!(
        story_call_kind_by_name(DROWN_PACK, DROWN_CALLBACK),
        StoryCallKind::Story
    );
}

#[test]
fn every_own_fall_and_respawn_callback_stays_with_its_player() {
    for name in OWN_FALL_AND_RESPAWN_CALLBACKS {
        assert!(is_own_body_callback_name(name), "{name}");
        assert_eq!(SceneScope::of(name), SceneScope::LocalScene, "{name}");
    }
}

#[test]
fn fight_field_fall_traps_keep_their_scope() {
    for name in FIGHT_FIELD_FALL_TRAPS_LEFT_AS_THEY_WERE {
        assert!(!is_own_body_callback_name(name), "{name}");
        assert_eq!(SceneScope::of(name), SceneScope::SharedProgress, "{name}");
    }
}

#[test]
fn the_hosts_drowning_is_never_sent_to_the_guest() {
    let mut ledger = StoryRelayLedger::new();
    let host = fire_facts(FollowRole::Host, DROWN_CALLBACK);
    assert_eq!(
        ledger.on_fire(DROWN_ARGUMENT_POINTER, &host, DROWN_KEY, 0),
        FireAction::RunLocalSceneOnly
    );
}

#[test]
fn the_guests_drowning_is_never_asked_of_the_host() {
    let mut ledger = StoryRelayLedger::new();
    let guest = fire_facts(FollowRole::Guest, DROWN_CALLBACK);
    assert_eq!(
        ledger.on_fire(DROWN_ARGUMENT_POINTER, &guest, DROWN_KEY, 0),
        FireAction::RunLocalSceneOnly
    );
}

#[test]
fn a_partners_copy_of_a_drowning_is_never_run() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_run(&run_facts(FollowRole::Guest, DROWN_CALLBACK), DROWN_KEY, 0),
        RunAction::Drop(DropReason::LocalScene)
    );
    assert_eq!(
        ledger.on_trigger(&run_facts(FollowRole::Host, DROWN_CALLBACK), DROWN_KEY, 0),
        TriggerAction::Drop(DropReason::LocalScene)
    );
}

#[test]
fn the_guests_drown_count_flag_still_reaches_the_host() {
    let scope = SceneScope::of(DROWN_CALLBACK);
    assert!(watches_scene_writes(
        FollowRole::Guest,
        true,
        scope,
        FireAction::RunLocalSceneOnly
    ));
    assert!(is_scene_write_for_the_host(StoryChange::Flag {
        array: StoryArray::SnowFlags,
        id: DROWN_COUNT_SNOW_FLAG,
        on: true,
    }));
}
