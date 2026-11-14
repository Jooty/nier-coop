use replicant_game::stay_together::FollowRole;
use replicant_game::story_calls::{SceneScope, StoryCallKind, name_hash};
use replicant_game::story_relay::{
    CallKey, DropReason, FireAction, FireFacts, OWN_STEP_SENT_REFUSES_ITS_TRIGGER_FOR_MS, RunFacts,
    STORY_CALL_DEDUPE_WINDOW_MS, StoryRelayLedger, TriggerAction,
};

const STEP: &str = "EID_1410_a0100g_21_04";
const STEP_HASH_IN_THE_RUN: u64 = 0x850e_cd75_aabb_9def;
const NO_ARGUMENTS_HASH_IN_THE_RUN: u64 = 0xcbf2_9ce4_8422_2325;
const HOST_SENT_THE_STEP_MS: u64 = 34_378;
const GUEST_RAN_THE_HOSTS_COPY_MS: u64 = 37_404;
const HOST_TOOK_THE_TRIGGER_MS: u64 = 37_420;
const SLOT: u64 = 0x1000;
const _: () =
    assert!(HOST_TOOK_THE_TRIGGER_MS - HOST_SENT_THE_STEP_MS > STORY_CALL_DEDUPE_WINDOW_MS);

fn key() -> CallKey {
    CallKey {
        function: name_hash(STEP),
        args: NO_ARGUMENTS_HASH_IN_THE_RUN,
    }
}

fn fire_facts(role: FollowRole, partners_copy: bool) -> FireFacts {
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
        each_run_counts: false,
        partners_copy,
    }
}

fn run_facts(each_run_counts: bool) -> RunFacts {
    RunFacts {
        enabled: true,
        role: FollowRole::Host,
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
fn the_step_of_the_run_is_the_one_the_logs_name() {
    assert_eq!(name_hash(STEP), STEP_HASH_IN_THE_RUN);
}

#[test]
fn the_hosts_step_copy_run_on_the_guest_is_never_asked_of_the_host() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_fire(
            SLOT,
            &fire_facts(FollowRole::Guest, true),
            key(),
            GUEST_RAN_THE_HOSTS_COPY_MS
        ),
        FireAction::RunLocally
    );
}

#[test]
fn the_guests_own_fire_of_a_story_step_is_still_asked_of_the_host() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_fire(
            SLOT,
            &fire_facts(FollowRole::Guest, false),
            key(),
            GUEST_RAN_THE_HOSTS_COPY_MS
        ),
        FireAction::SuppressAndAsk
    );
}

#[test]
fn the_guests_copy_run_on_the_host_is_never_announced_back() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_fire(SLOT, &fire_facts(FollowRole::Host, true), key(), 0),
        FireAction::RunLocally
    );
}

#[test]
fn a_trigger_of_a_step_the_host_sent_is_refused_past_the_dedupe_window() {
    let mut ledger = StoryRelayLedger::new();
    ledger.note_step_sent(name_hash(STEP), HOST_SENT_THE_STEP_MS);
    assert_eq!(
        ledger.on_trigger(&run_facts(false), key(), HOST_TOOK_THE_TRIGGER_MS),
        TriggerAction::Drop(DropReason::Duplicate)
    );
}

#[test]
fn a_step_the_host_announced_refuses_the_guests_trigger_of_it() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_fire(
            SLOT,
            &fire_facts(FollowRole::Host, false),
            key(),
            HOST_SENT_THE_STEP_MS
        ),
        FireAction::RunAndAnnounce
    );
    assert_eq!(
        ledger.on_trigger(&run_facts(false), key(), HOST_TOOK_THE_TRIGGER_MS),
        TriggerAction::Drop(DropReason::Duplicate)
    );
}

#[test]
fn a_trigger_long_after_the_send_is_accepted_again() {
    let mut ledger = StoryRelayLedger::new();
    ledger.note_step_sent(name_hash(STEP), HOST_SENT_THE_STEP_MS);
    assert_eq!(
        ledger.on_trigger(
            &run_facts(false),
            key(),
            HOST_SENT_THE_STEP_MS + OWN_STEP_SENT_REFUSES_ITS_TRIGGER_FOR_MS
        ),
        TriggerAction::InjectAndBroadcast
    );
}

#[test]
fn a_counted_talk_the_host_sent_is_still_run_for_the_guest() {
    let mut ledger = StoryRelayLedger::new();
    ledger.note_step_sent(name_hash(STEP), HOST_SENT_THE_STEP_MS);
    assert_eq!(
        ledger.on_trigger(&run_facts(true), key(), HOST_TOOK_THE_TRIGGER_MS),
        TriggerAction::InjectAndBroadcast
    );
}

#[test]
fn a_trigger_of_another_step_is_not_refused_by_the_send() {
    let mut ledger = StoryRelayLedger::new();
    ledger.note_step_sent(name_hash(STEP), HOST_SENT_THE_STEP_MS);
    let other = CallKey {
        function: name_hash("EID_1410_a0100g_22"),
        args: NO_ARGUMENTS_HASH_IN_THE_RUN,
    };
    assert_eq!(
        ledger.on_trigger(&run_facts(false), other, HOST_TOOK_THE_TRIGGER_MS),
        TriggerAction::InjectAndBroadcast
    );
}
