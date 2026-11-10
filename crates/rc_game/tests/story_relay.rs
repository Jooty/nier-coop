use rc_game::stay_together::FollowRole;
use rc_game::story_calls::{
    PartnerPlace, SAME_PLACE_RANGE_UNITS, STORY_CALL_TABLE, SceneScope, StoryCallKind, Whereabouts,
};
use rc_game::story_relay::{
    ARG_STRIDE_BYTES, ARGS_HEADER_BYTES, ArgsRefusal, CallKey, DropReason, FireAction, FireFacts,
    HELD_TRIGGER_ANSWER_TIMEOUT_MS, HeldTimeoutVerdict, INJECTED_CALL_MARK_LIFETIME_MS,
    InjectedMatch, RAN_LOCALLY_AFTER_TIMEOUT_RECORD_MS, RefusedAction, RunAction, RunFacts,
    STORY_CALL_DEDUPE_WINDOW_MS, StoryRelayLedger, TriggerAction, args_block_len, args_hash,
    held_timeout_verdict, pack_hash_of_phase, read_args_block, relay_kind, write_args_block,
};
use rc_game::story_writes::ScriptCallTracker;
use rc_proto::{MAX_STORY_CALL_ARGS, StoryCallArg, StoryRefusalReason};

const KEY: CallKey = CallKey {
    function: 7,
    args: 9,
};
const OTHER_KEY: CallKey = CallKey {
    function: 8,
    args: 9,
};
const SLOT: u64 = 0x1000;

fn facts(role: FollowRole) -> FireFacts {
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
        can_gather_partner: true,
    }
}

fn run_facts(role: FollowRole) -> RunFacts {
    RunFacts {
        enabled: true,
        role,
        local_in_play: true,
        local_phase: Some(4),
        call_phase: 4,
        known_story_call: true,
        scope: SceneScope::SharedProgress,
        partner_here: true,
    }
}

#[test]
fn arguments_round_trip_through_the_game_block() {
    let args = vec![
        StoryCallArg::Int(-5),
        StoryCallArg::Number(0x3FF0_0000_0000_0000),
        StoryCallArg::Text(b"hello".to_vec()),
        StoryCallArg::Text(Vec::new()),
    ];
    let block = write_args_block(&args);
    assert_eq!(block.len(), ARGS_HEADER_BYTES + 4 * ARG_STRIDE_BYTES);
    assert_eq!(read_args_block(&block), Ok(args));
}

#[test]
fn a_call_without_arguments_is_an_empty_block() {
    assert_eq!(read_args_block(&write_args_block(&[])), Ok(Vec::new()));
}

#[test]
fn pointer_arguments_and_unknown_tags_are_refused() {
    let mut block = write_args_block(&[StoryCallArg::Int(1)]);
    block[ARGS_HEADER_BYTES..ARGS_HEADER_BYTES + 4].copy_from_slice(&3_i32.to_le_bytes());
    assert_eq!(read_args_block(&block), Err(ArgsRefusal::Pointer));
    block[ARGS_HEADER_BYTES..ARGS_HEADER_BYTES + 4].copy_from_slice(&9_i32.to_le_bytes());
    assert_eq!(read_args_block(&block), Err(ArgsRefusal::UnknownTag));
}

#[test]
fn numbers_that_are_not_script_numbers_are_refused() {
    for bits in [
        0x0000_7FF6_1234_5678_u64,
        0x7FF0_0000_0000_0000,
        0x7FF8_0000_0000_0000,
        0x0000_0000_0000_0001,
    ] {
        let block = write_args_block(&[StoryCallArg::Number(bits)]);
        assert_eq!(
            read_args_block(&block),
            Err(ArgsRefusal::MachineLocalNumber)
        );
    }
    let zero = write_args_block(&[StoryCallArg::Number(0)]);
    assert_eq!(read_args_block(&zero), Ok(vec![StoryCallArg::Number(0)]));
}

#[test]
fn too_many_arguments_a_long_text_or_a_short_block_are_refused() {
    assert_eq!(args_block_len(-1), None);
    assert_eq!(args_block_len(MAX_STORY_CALL_ARGS as i32 + 1), None);
    let too_many = (MAX_STORY_CALL_ARGS as i32 + 1).to_le_bytes();
    assert_eq!(read_args_block(&too_many), Err(ArgsRefusal::TooMany));
    let block = write_args_block(&[StoryCallArg::Int(1)]);
    assert_eq!(
        read_args_block(&block[..ARGS_HEADER_BYTES + 10]),
        Err(ArgsRefusal::Truncated)
    );
    assert_eq!(read_args_block(&[1]), Err(ArgsRefusal::Truncated));
    let mut long = write_args_block(&[StoryCallArg::Text(b"x".to_vec())]);
    for byte in &mut long[ARGS_HEADER_BYTES + 0x10..ARGS_HEADER_BYTES + 0x10 + 60] {
        *byte = b'y';
    }
    assert_eq!(read_args_block(&long), Err(ArgsRefusal::TextTooLong));
}

#[test]
fn equal_arguments_hash_equal_and_different_ones_do_not() {
    let one = [StoryCallArg::Int(1), StoryCallArg::Text(b"a".to_vec())];
    let same = [StoryCallArg::Int(1), StoryCallArg::Text(b"a".to_vec())];
    let other = [StoryCallArg::Int(2), StoryCallArg::Text(b"a".to_vec())];
    assert_eq!(args_hash(&one), args_hash(&same));
    assert_ne!(args_hash(&one), args_hash(&other));
    assert_ne!(
        args_hash(&[StoryCallArg::Int(1)]),
        args_hash(&[StoryCallArg::Number(1)])
    );
}

#[test]
fn an_unknown_pack_has_no_kind_and_a_known_pack_falls_back_to_local() {
    assert_eq!(relay_kind(0, 0), None);
    let (pack, function, kind) = STORY_CALL_TABLE[0];
    assert_eq!(relay_kind(pack, function), Some(kind));
    assert_eq!(relay_kind(pack, u64::MAX), Some(StoryCallKind::Local));
    assert_eq!(
        pack_hash_of_phase("A"),
        rc_game::story_calls::name_hash("A")
    );
}

#[test]
fn only_story_calls_in_play_with_a_partner_are_relayed() {
    let mut ledger = StoryRelayLedger::new();
    let guest = |change: fn(&mut FireFacts)| {
        let mut value = facts(FollowRole::Guest);
        change(&mut value);
        value
    };
    let cases = [
        guest(|value| value.enabled = false),
        guest(|value| value.connected = false),
        guest(|value| value.local_in_play = false),
        guest(|value| value.partner_in_same_phase = false),
        guest(|value| value.kind = None),
        guest(|value| value.kind = Some(StoryCallKind::Local)),
        guest(|value| value.kind = Some(StoryCallKind::Container)),
        guest(|value| value.owned_elsewhere = true),
        guest(|value| value.args_relayable = false),
        guest(|value| value.role = FollowRole::Unknown),
    ];
    for case in cases {
        assert_eq!(ledger.on_fire(SLOT, &case, KEY, 0), FireAction::RunLocally);
    }
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 0),
        FireAction::SuppressAndAsk
    );
}

#[test]
fn the_host_runs_its_own_call_and_announces_it() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Host), KEY, 0),
        FireAction::RunAndAnnounce
    );
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Host), OTHER_KEY, 0),
        FireAction::RunAndAnnounce
    );
}

#[test]
fn a_trigger_that_follows_the_hosts_own_run_is_a_duplicate() {
    let mut ledger = StoryRelayLedger::new();
    ledger.on_fire(SLOT, &facts(FollowRole::Host), KEY, 100);
    assert_eq!(
        ledger.on_trigger(&run_facts(FollowRole::Host), KEY, 900),
        TriggerAction::Drop(DropReason::Duplicate)
    );
    assert_eq!(
        ledger.on_trigger(&run_facts(FollowRole::Host), OTHER_KEY, 900),
        TriggerAction::InjectAndBroadcast
    );
}

#[test]
fn a_second_trigger_inside_the_window_is_dropped_and_one_after_it_is_accepted() {
    let mut ledger = StoryRelayLedger::new();
    let host = run_facts(FollowRole::Host);
    assert_eq!(
        ledger.on_trigger(&host, KEY, 0),
        TriggerAction::InjectAndBroadcast
    );
    assert_eq!(
        ledger.on_trigger(&host, KEY, STORY_CALL_DEDUPE_WINDOW_MS - 1),
        TriggerAction::Drop(DropReason::Duplicate)
    );
    assert_eq!(
        ledger.on_trigger(&host, KEY, STORY_CALL_DEDUPE_WINDOW_MS),
        TriggerAction::InjectAndBroadcast
    );
}

#[test]
fn the_hosts_own_fire_after_an_accepted_trigger_is_dropped_once() {
    let mut ledger = StoryRelayLedger::new();
    ledger.on_trigger(&run_facts(FollowRole::Host), KEY, 0);
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Host), KEY, 500),
        FireAction::DropDuplicate
    );
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Host), KEY, 600),
        FireAction::RunAndAnnounce
    );
}

#[test]
fn an_injected_call_passes_once_and_is_not_announced_again() {
    let mut ledger = StoryRelayLedger::new();
    ledger.note_injected(SLOT, KEY, 0);
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Host), KEY, 10),
        FireAction::RunInjected(InjectedMatch::Pointer)
    );
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Host), KEY, 20),
        FireAction::RunAndAnnounce
    );
}

#[test]
fn an_injected_call_passes_even_when_relaying_is_off() {
    let mut ledger = StoryRelayLedger::new();
    ledger.note_injected(SLOT, KEY, 0);
    let mut off = facts(FollowRole::Guest);
    off.enabled = false;
    assert_eq!(
        ledger.on_fire(SLOT, &off, KEY, 10),
        FireAction::RunInjected(InjectedMatch::Pointer)
    );
}

#[test]
fn an_injected_mark_expires() {
    let mut ledger = StoryRelayLedger::new();
    ledger.note_injected(SLOT, KEY, 0);
    assert_eq!(
        ledger.on_fire(
            SLOT,
            &facts(FollowRole::Guest),
            KEY,
            INJECTED_CALL_MARK_LIFETIME_MS
        ),
        FireAction::SuppressAndAsk
    );
}

#[test]
fn the_guests_own_fire_after_the_hosts_run_is_dropped_once() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_run(&run_facts(FollowRole::Guest), KEY, 0),
        RunAction::Inject
    );
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 300),
        FireAction::DropDuplicate
    );
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 400),
        FireAction::SuppressAndAsk
    );
}

#[test]
fn a_message_for_another_phase_is_dropped_by_both_sides() {
    let mut ledger = StoryRelayLedger::new();
    let mut guest = run_facts(FollowRole::Guest);
    guest.call_phase = 5;
    assert_eq!(
        ledger.on_run(&guest, KEY, 0),
        RunAction::Drop(DropReason::WrongPhase)
    );
    let mut host = run_facts(FollowRole::Host);
    host.local_phase = None;
    assert_eq!(
        ledger.on_trigger(&host, KEY, 0),
        TriggerAction::Drop(DropReason::WrongPhase)
    );
}

#[test]
fn messages_are_dropped_when_off_in_the_wrong_role_or_not_in_play() {
    let mut ledger = StoryRelayLedger::new();
    let mut off = run_facts(FollowRole::Host);
    off.enabled = false;
    assert_eq!(
        ledger.on_trigger(&off, KEY, 0),
        TriggerAction::Drop(DropReason::Disabled)
    );
    assert_eq!(
        ledger.on_trigger(&run_facts(FollowRole::Guest), KEY, 0),
        TriggerAction::Drop(DropReason::WrongRole)
    );
    assert_eq!(
        ledger.on_run(&run_facts(FollowRole::Host), KEY, 0),
        RunAction::Drop(DropReason::WrongRole)
    );
    let mut unknown = run_facts(FollowRole::Guest);
    unknown.known_story_call = false;
    assert_eq!(
        ledger.on_run(&unknown, KEY, 0),
        RunAction::Drop(DropReason::NotAStoryCall)
    );
    let mut loading = run_facts(FollowRole::Guest);
    loading.local_in_play = false;
    assert_eq!(
        ledger.on_run(&loading, KEY, 0),
        RunAction::Drop(DropReason::NotInPlay)
    );
}

#[test]
fn a_call_a_talk_tag_queued_can_be_asked_about_without_consuming_it() {
    let mut tracker = ScriptCallTracker::new();
    tracker.note_talk_tag_call(b"Select01", 100);
    assert!(tracker.queued_by_talk_tag(b"Select01", 150));
    assert!(tracker.queued_by_talk_tag(b"Select01", 160));
    assert!(!tracker.queued_by_talk_tag(b"Other", 150));
    assert!(!tracker.queued_by_talk_tag(b"", 150));
    assert!(!tracker.queued_by_talk_tag(b"Select01", 100 + 30_000));
}

#[test]
fn the_guests_held_call_is_run_here_after_a_refusal_but_only_forgotten_after_a_duplicate() {
    let mut ledger = StoryRelayLedger::new();
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 0),
        FireAction::SuppressAndAsk
    );
    assert_eq!(
        ledger.on_refused(KEY, StoryRefusalReason::WrongPhase, 0),
        RefusedAction::RunHere
    );
    assert_eq!(
        ledger.on_refused(KEY, StoryRefusalReason::WrongPhase, 0),
        RefusedAction::NotHeld
    );
    ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 10);
    assert_eq!(
        ledger.on_refused(KEY, StoryRefusalReason::Duplicate, 0),
        RefusedAction::ForgetOnly
    );
}

#[test]
fn a_held_call_with_no_answer_comes_back_after_the_timeout_once() {
    let mut ledger = StoryRelayLedger::new();
    ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 100);
    assert!(
        ledger
            .take_timed_out_held(100 + HELD_TRIGGER_ANSWER_TIMEOUT_MS - 1)
            .is_empty()
    );
    assert_eq!(
        ledger.take_timed_out_held(100 + HELD_TRIGGER_ANSWER_TIMEOUT_MS),
        vec![KEY]
    );
    assert!(
        ledger
            .take_timed_out_held(100 + 2 * HELD_TRIGGER_ANSWER_TIMEOUT_MS)
            .is_empty()
    );
}

#[test]
fn the_hosts_run_clears_the_guests_held_call() {
    let mut ledger = StoryRelayLedger::new();
    ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 0);
    ledger.on_run(&run_facts(FollowRole::Guest), KEY, 50);
    assert!(
        ledger
            .take_timed_out_held(HELD_TRIGGER_ANSWER_TIMEOUT_MS * 2)
            .is_empty()
    );
}

#[test]
fn a_natural_fire_within_the_window_after_an_injected_run_is_dropped_on_the_guest() {
    let mut ledger = StoryRelayLedger::new();
    ledger.on_run(&run_facts(FollowRole::Guest), KEY, 0);
    ledger.note_injected(SLOT, KEY, 0);
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 10),
        FireAction::RunInjected(InjectedMatch::Pointer)
    );
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 100),
        FireAction::DropDuplicate
    );
}

#[test]
fn a_natural_fire_within_the_window_after_an_injected_run_is_dropped_on_the_host_then_runs_after_it()
 {
    let mut ledger = StoryRelayLedger::new();
    ledger.on_trigger(&run_facts(FollowRole::Host), KEY, 0);
    ledger.note_injected(SLOT, KEY, 0);
    ledger.on_fire(SLOT, &facts(FollowRole::Host), KEY, 10);
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Host), KEY, 100),
        FireAction::DropDuplicate
    );
    ledger.on_trigger(&run_facts(FollowRole::Host), KEY, 5_000);
    ledger.note_injected(SLOT, KEY, 5_000);
    ledger.on_fire(SLOT, &facts(FollowRole::Host), KEY, 5_010);
    assert_eq!(
        ledger.on_fire(
            SLOT,
            &facts(FollowRole::Host),
            KEY,
            5_010 + STORY_CALL_DEDUPE_WINDOW_MS
        ),
        FireAction::RunAndAnnounce
    );
}

#[test]
fn a_zero_argument_call_is_recognised_by_function_and_argument_hash() {
    let zero_args = CallKey {
        function: 77,
        args: args_hash(&[]),
    };
    let mut ledger = StoryRelayLedger::new();
    ledger.note_injected(SLOT, zero_args, 0);
    assert_eq!(
        ledger.on_fire(0, &facts(FollowRole::Host), zero_args, 20),
        FireAction::RunInjected(InjectedMatch::Key {
            queued_arguments: SLOT
        })
    );
    assert_eq!(
        ledger.on_fire(0, &facts(FollowRole::Host), zero_args, 30),
        FireAction::RunAndAnnounce
    );
}

#[test]
fn the_key_mark_only_matches_inside_the_tick_that_injected_and_not_for_another_call() {
    let mut ledger = StoryRelayLedger::new();
    ledger.begin_tick();
    ledger.note_injected(SLOT, KEY, 0);
    assert_eq!(
        ledger.on_fire(0, &facts(FollowRole::Host), OTHER_KEY, 10),
        FireAction::RunAndAnnounce
    );
    ledger.begin_tick();
    assert_eq!(
        ledger.on_fire(0, &facts(FollowRole::Host), KEY, 20),
        FireAction::RunAndAnnounce
    );
    ledger.note_injected(SLOT, KEY, 30);
    assert_eq!(
        ledger.on_fire(SLOT + 8, &facts(FollowRole::Host), KEY, 40),
        FireAction::RunInjected(InjectedMatch::Key {
            queued_arguments: SLOT
        })
    );
}

#[test]
fn a_late_answer_after_the_guest_ran_its_held_call_is_ignored_once() {
    let mut ledger = StoryRelayLedger::new();
    ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 0);
    assert_eq!(
        ledger.take_timed_out_held(HELD_TRIGGER_ANSWER_TIMEOUT_MS),
        vec![KEY]
    );
    ledger.note_ran_locally(KEY, HELD_TRIGGER_ANSWER_TIMEOUT_MS);
    assert_eq!(
        ledger.on_run(&run_facts(FollowRole::Guest), KEY, 2_000),
        RunAction::IgnoreLate
    );
    assert_eq!(
        ledger.on_run(&run_facts(FollowRole::Guest), KEY, 2_100),
        RunAction::Inject
    );
    ledger.note_ran_locally(KEY, 3_000);
    assert_eq!(
        ledger.on_refused(KEY, StoryRefusalReason::WrongPhase, 4_000),
        RefusedAction::IgnoreLate
    );
}

#[test]
fn the_ran_here_record_expires() {
    let mut ledger = StoryRelayLedger::new();
    ledger.note_ran_locally(KEY, 0);
    assert_eq!(
        ledger.on_run(
            &run_facts(FollowRole::Guest),
            KEY,
            RAN_LOCALLY_AFTER_TIMEOUT_RECORD_MS
        ),
        RunAction::Inject
    );
}

#[test]
fn a_timed_out_call_waits_while_loading_and_drops_only_on_a_phase_change() {
    assert_eq!(
        held_timeout_verdict(true, Some(4), 4),
        HeldTimeoutVerdict::RunHere
    );
    assert_eq!(
        held_timeout_verdict(false, Some(4), 4),
        HeldTimeoutVerdict::KeepWaiting
    );
    assert_eq!(
        held_timeout_verdict(false, None, 4),
        HeldTimeoutVerdict::KeepWaiting
    );
    assert_eq!(
        held_timeout_verdict(false, Some(5), 4),
        HeldTimeoutVerdict::Drop
    );
    assert_eq!(
        held_timeout_verdict(true, Some(5), 4),
        HeldTimeoutVerdict::Drop
    );
}

#[test]
fn ending_the_session_forgets_every_mark_and_hold() {
    let mut ledger = StoryRelayLedger::new();
    ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 0);
    ledger.note_injected(SLOT, OTHER_KEY, 0);
    ledger.note_ran_locally(KEY, 0);
    assert!(!ledger.is_idle());
    ledger.clear_session();
    assert!(ledger.is_idle());
    assert!(
        ledger
            .take_timed_out_held(HELD_TRIGGER_ANSWER_TIMEOUT_MS)
            .is_empty()
    );
}

#[test]
fn every_drop_reason_has_a_wire_reason() {
    assert_eq!(
        StoryRefusalReason::from(DropReason::Duplicate),
        StoryRefusalReason::Duplicate
    );
    assert_eq!(
        StoryRefusalReason::from(DropReason::WrongPhase),
        StoryRefusalReason::WrongPhase
    );
}

#[test]
fn the_guests_own_fire_that_takes_the_injection_leaves_no_token_to_drop_a_later_fire() {
    let mut ledger = StoryRelayLedger::new();
    ledger.on_run(&run_facts(FollowRole::Guest), KEY, 0);
    ledger.note_injected(SLOT, KEY, 0);
    assert_eq!(
        ledger.on_fire(SLOT + 8, &facts(FollowRole::Guest), KEY, 10),
        FireAction::RunInjected(InjectedMatch::Key {
            queued_arguments: SLOT
        })
    );
    assert_eq!(
        ledger.on_fire(SLOT, &facts(FollowRole::Guest), KEY, 20),
        FireAction::SuppressAndAsk
    );
}

#[test]
fn a_local_scene_call_runs_only_where_it_fired() {
    let mut ledger = StoryRelayLedger::new();
    for role in [FollowRole::Host, FollowRole::Guest] {
        let local = FireFacts {
            scope: SceneScope::LocalScene,
            ..facts(role)
        };
        assert_eq!(
            ledger.on_fire(SLOT, &local, KEY, 0),
            FireAction::RunLocalSceneOnly
        );
    }
}

#[test]
fn a_relayed_local_scene_call_is_never_injected() {
    let mut ledger = StoryRelayLedger::new();
    let host = RunFacts {
        scope: SceneScope::LocalScene,
        ..run_facts(FollowRole::Host)
    };
    assert_eq!(
        ledger.on_trigger(&host, KEY, 0),
        TriggerAction::Drop(DropReason::LocalScene)
    );
    let guest = RunFacts {
        scope: SceneScope::LocalScene,
        ..run_facts(FollowRole::Guest)
    };
    assert_eq!(
        ledger.on_run(&guest, KEY, 0),
        RunAction::Drop(DropReason::LocalScene)
    );
}

#[test]
fn a_shared_scene_runs_alone_when_the_partner_is_elsewhere() {
    let mut ledger = StoryRelayLedger::new();
    let away = FireFacts {
        scope: SceneScope::SharedScene,
        partner_here: false,
        ..facts(FollowRole::Guest)
    };
    assert_eq!(
        ledger.on_fire(SLOT, &away, KEY, 0),
        FireAction::RunWithoutPartner
    );
    let guest = RunFacts {
        scope: SceneScope::SharedScene,
        partner_here: false,
        ..run_facts(FollowRole::Guest)
    };
    assert_eq!(
        ledger.on_run(&guest, KEY, 0),
        RunAction::Drop(DropReason::Elsewhere)
    );
}

#[test]
fn shared_progress_runs_on_both_wherever_the_partner_stands() {
    let mut ledger = StoryRelayLedger::new();
    let guest = RunFacts {
        partner_here: false,
        ..run_facts(FollowRole::Guest)
    };
    assert_eq!(ledger.on_run(&guest, KEY, 0), RunAction::Inject);
}

#[test]
fn local_scene_and_elsewhere_refusals_go_on_the_wire_as_known_reasons() {
    assert_eq!(
        StoryRefusalReason::from(DropReason::LocalScene),
        StoryRefusalReason::NotAStoryCall
    );
    assert_eq!(
        StoryRefusalReason::from(DropReason::Elsewhere),
        StoryRefusalReason::WrongPhase
    );
}

fn at(phase: u16, x: f32) -> Option<Whereabouts> {
    Some(Whereabouts {
        phase,
        position: [x, 0.0, 0.0],
    })
}

#[test]
fn the_partner_is_in_the_same_place_within_range_in_the_same_phase() {
    assert_eq!(
        PartnerPlace::judge(at(3, 0.0), at(3, 26.4)),
        PartnerPlace::Same
    );
    assert_eq!(
        PartnerPlace::judge(at(3, 0.0), at(3, SAME_PLACE_RANGE_UNITS + 1.0)),
        PartnerPlace::TooFar {
            units: SAME_PLACE_RANGE_UNITS + 1.0
        }
    );
    assert_eq!(
        PartnerPlace::judge(at(3, 0.0), at(4, 0.0)),
        PartnerPlace::OtherPhase
    );
    assert_eq!(PartnerPlace::judge(at(3, 0.0), None), PartnerPlace::Unknown);
    assert!(!PartnerPlace::Unknown.is_same());
}
