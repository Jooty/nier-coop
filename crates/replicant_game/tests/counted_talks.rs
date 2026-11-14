use replicant_game::counted_talks::{
    COUNTED_TALK_CALLBACKS, counted_talk_callback, talk_keeps_the_call,
};
use replicant_game::stay_together::FollowRole;
use replicant_game::story_calls::{SceneScope, StoryCallKind, name_hash, story_call_kind_by_name};
use replicant_game::story_relay::{
    CallKey, FireAction, FireFacts, RunAction, RunFacts, StoryRelayLedger, TriggerAction,
};
use replicant_game::story_writes::ScriptCallTracker;

const FIRST_COUNTED_PACK: &str = "A_DESERT_TOWN_01";
const FIRST_COUNTED_CALLBACK: &str = "Count_Talk_Num";
const SECOND_COUNTED_PACK: &str = "A_SEASIDE_TOWN_01";
const SECOND_COUNTED_CALLBACK: &str = "SeaSide_NPC_Talk_End";
const KEY: CallKey = CallKey {
    function: 7,
    args: 9,
};
const SLOT: u64 = 0x1000;
const OTHER_SLOT: u64 = 0x2000;

fn fire_facts(role: FollowRole, each_run_counts: bool) -> FireFacts {
    FireFacts {
        enabled: true,
        connected: true,
        role,
        local_in_play: true,
        partner_in_same_phase: true,
        kind: Some(StoryCallKind::Story),
        scope: SceneScope::SharedProgress,
        partner_here: true,
        owned_elsewhere: false,
        args_relayable: true,
        partner_too_far: false,
        can_gather_partner: false,
        each_run_counts,
        partners_copy: false,
    }
}

fn run_facts(role: FollowRole, each_run_counts: bool) -> RunFacts {
    RunFacts {
        enabled: true,
        role,
        local_in_play: true,
        local_phase: Some(7),
        call_phase: 7,
        known_story_call: true,
        scope: SceneScope::SharedProgress,
        partner_here: true,
        each_run_counts,
    }
}

#[test]
fn each_counted_talk_callback_is_found_only_in_its_own_pack() {
    let first = counted_talk_callback(
        name_hash(FIRST_COUNTED_PACK),
        name_hash(FIRST_COUNTED_CALLBACK),
    );
    assert_eq!(
        first.map(|callback| callback.count_global),
        Some("Count_Num")
    );
    let second = counted_talk_callback(
        name_hash(SECOND_COUNTED_PACK),
        name_hash(SECOND_COUNTED_CALLBACK),
    );
    assert_eq!(
        second.map(|callback| callback.count_global),
        Some("NPC_Talk_Count")
    );
    assert_eq!(
        counted_talk_callback(
            name_hash(SECOND_COUNTED_PACK),
            name_hash(FIRST_COUNTED_CALLBACK)
        ),
        None
    );
}

#[test]
fn every_counted_talk_callback_is_a_story_call_so_the_relay_carries_it() {
    for callback in COUNTED_TALK_CALLBACKS {
        assert_eq!(
            story_call_kind_by_name(callback.pack, callback.function),
            StoryCallKind::Story,
            "{}",
            callback.function
        );
    }
}

#[test]
fn a_counted_talk_callback_is_not_kept_by_the_talk_that_queued_it() {
    let pack = Some(name_hash(FIRST_COUNTED_PACK));
    assert!(!talk_keeps_the_call(
        true,
        pack,
        name_hash(FIRST_COUNTED_CALLBACK)
    ));
    assert!(talk_keeps_the_call(
        true,
        None,
        name_hash(FIRST_COUNTED_CALLBACK)
    ));
}

#[test]
fn talk_tag_callbacks_that_count_nothing_for_the_story_stay_with_the_talker() {
    let pauses = talk_keeps_the_call(
        true,
        Some(name_hash("B_MOUNT_ROBOT_10")),
        name_hash("Check_PauseOff_Timing"),
    );
    let picks = talk_keeps_the_call(
        true,
        Some(name_hash("C_FOREST_FIELD_01")),
        name_hash("Labyrinth_Divergence"),
    );
    assert!(pauses && picks);
    assert!(!talk_keeps_the_call(
        false,
        Some(name_hash("B_MOUNT_ROBOT_10")),
        name_hash("Check_PauseOff_Timing")
    ));
}

#[test]
fn the_host_runs_its_own_counted_talk_right_after_running_the_guests() {
    let mut ledger = StoryRelayLedger::new();
    let host_run = run_facts(FollowRole::Host, true);
    assert_eq!(
        ledger.on_trigger(&host_run, KEY, 100),
        TriggerAction::InjectAndBroadcast
    );
    let own = fire_facts(FollowRole::Host, true);
    assert_eq!(
        ledger.on_fire(SLOT, &own, KEY, 200),
        FireAction::RunAndAnnounce
    );

    let mut uncounted = StoryRelayLedger::new();
    uncounted.on_trigger(&run_facts(FollowRole::Host, false), KEY, 100);
    assert_eq!(
        uncounted.on_fire(SLOT, &fire_facts(FollowRole::Host, false), KEY, 200),
        FireAction::DropDuplicate
    );
}

#[test]
fn the_host_takes_every_counted_trigger_of_the_guest_after_its_own_talk() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_fire(SLOT, &fire_facts(FollowRole::Host, true), KEY, 100),
        FireAction::RunAndAnnounce
    );
    let host_run = run_facts(FollowRole::Host, true);
    assert_eq!(
        ledger.on_trigger(&host_run, KEY, 200),
        TriggerAction::InjectAndBroadcast
    );
    assert_eq!(
        ledger.on_trigger(&host_run, KEY, 300),
        TriggerAction::InjectAndBroadcast
    );
}

#[test]
fn the_guest_asks_the_host_for_its_counted_talk_right_after_replaying_the_hosts() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_run(&run_facts(FollowRole::Guest, true), KEY, 100),
        RunAction::Inject
    );
    assert_eq!(
        ledger.on_fire(OTHER_SLOT, &fire_facts(FollowRole::Guest, true), KEY, 200),
        FireAction::SuppressAndAsk
    );

    let mut uncounted = StoryRelayLedger::new();
    uncounted.on_run(&run_facts(FollowRole::Guest, false), KEY, 100);
    assert_eq!(
        uncounted.on_fire(OTHER_SLOT, &fire_facts(FollowRole::Guest, false), KEY, 200),
        FireAction::DropDuplicate
    );
}

#[test]
fn the_guest_replays_every_counted_run_of_the_host() {
    let mut ledger = StoryRelayLedger::new();
    let guest_run = run_facts(FollowRole::Guest, true);
    assert_eq!(ledger.on_run(&guest_run, KEY, 100), RunAction::Inject);
    assert_eq!(ledger.on_run(&guest_run, KEY, 300), RunAction::Inject);
}

#[test]
fn a_host_run_arriving_after_the_guest_counted_its_talk_alone_is_not_counted_twice() {
    let mut ledger = StoryRelayLedger::new();
    ledger.note_ran_locally(KEY, 100);
    assert_eq!(
        ledger.on_run(&run_facts(FollowRole::Guest, true), KEY, 200),
        RunAction::IgnoreLate
    );
}

#[test]
fn a_forgotten_talk_tag_call_starts_as_an_ordinary_call() {
    let mut tracker = ScriptCallTracker::new();
    tracker.note_talk_tag_call(FIRST_COUNTED_CALLBACK.as_bytes(), 100);
    tracker.forget_talk_tag_call(FIRST_COUNTED_CALLBACK.as_bytes());
    assert!(!tracker.queued_by_talk_tag(FIRST_COUNTED_CALLBACK.as_bytes(), 150));
    let started = tracker.start_call(FIRST_COUNTED_CALLBACK.as_bytes(), 200);
    assert_eq!(started.map(|call| call.from_talk_tag), Some(false));
}
