use replicant_game::fight_sync::TRAP_PRIVATE_GAME_FLAGS;
use replicant_game::stay_together::FollowRole;
use replicant_game::story_calls::{SceneScope, name_hash};
use replicant_game::story_progress::{
    FeedSend, STORY_DRIFT_GRACE_MS, StoryArrays, StoryFeed, StoryKey, StoryMirror,
};
use replicant_game::story_relay::FireAction;
use replicant_game::story_writes::{
    AREA_SCRATCH_GAME_FLAG_IDS, END_WATCH_SETTLE_MS, EndWatch, HOST_OWN_CHANGE_PRIORITY_MS,
    HostChangeLog, LOCAL_SCENE_CALL_START_WAIT_MAX_MS, LOCAL_SCENE_WRITE_WATCH_MAX_MS,
    MAX_TALK_QUEUED_NAMES, STORY_WRITE_HOLD_MS, SceneWriteWatch, ScriptCallTracker,
    TALK_CALLBACK_ATTRIBUTION_MAX_MS, TALK_QUEUED_NAME_LIFETIME_MS, host_write_outcome,
    is_scene_write_for_the_host, rebaseline_over_mirror_fixes, talk_tag_function_name,
    talker_write_batches, watches_scene_writes,
};
use replicant_proto::{MAX_STORY_CHANGES_PER_MESSAGE, StoryArray, StoryArrayPart, StoryChange};

fn flag(id: u16, on: bool) -> StoryChange {
    StoryChange::Flag {
        array: StoryArray::GameFlags,
        id,
        on,
    }
}

fn arrays_with(changes: &[StoryChange]) -> StoryArrays {
    let mut arrays = StoryArrays::default();
    for &change in changes {
        arrays.apply(change);
    }
    arrays
}

#[test]
fn the_function_a_tag_names_stops_at_the_first_comma() {
    assert_eq!(talk_tag_function_name(b"Select01,12,\"x\""), b"Select01");
    assert_eq!(talk_tag_function_name(b" Plain \0junk"), b"Plain");
    assert_eq!(talk_tag_function_name(b""), b"");
    let long = vec![b'a'; 200];
    assert_eq!(talk_tag_function_name(&long).len(), 64);
}

#[test]
fn a_call_a_talk_tag_queued_is_marked_once_and_other_calls_are_not() {
    let mut tracker = ScriptCallTracker::new();
    assert_eq!(tracker.running(), None);
    tracker.note_talk_tag_call(b"Select01", 100);
    let other = tracker.start_call(b"LoadScript", 150).unwrap();
    assert!(!other.from_talk_tag);
    assert!(!tracker.talk_callback_running(150));
    let talk = tracker.start_call(b"Select01", 200).unwrap();
    assert!(talk.from_talk_tag);
    assert_eq!(talk.name_hash, name_hash("Select01"));
    assert!(tracker.talk_callback_running(300));
    let again = tracker.start_call(b"Select01", 400).unwrap();
    assert!(!again.from_talk_tag);
    assert!(!tracker.talk_callback_running(450));
}

#[test]
fn a_talk_callback_stops_counting_after_the_cap_and_a_stale_queue_entry_is_forgotten() {
    let mut tracker = ScriptCallTracker::new();
    tracker.note_talk_tag_call(b"Select01", 0);
    tracker.start_call(b"Select01", 10);
    assert!(tracker.talk_callback_running(TALK_CALLBACK_ATTRIBUTION_MAX_MS - 1));
    assert!(!tracker.talk_callback_running(10 + TALK_CALLBACK_ATTRIBUTION_MAX_MS));
    tracker.note_talk_tag_call(b"Old", 0);
    let late = tracker
        .start_call(b"Old", TALK_QUEUED_NAME_LIFETIME_MS)
        .unwrap();
    assert!(!late.from_talk_tag);
    for number in 0..MAX_TALK_QUEUED_NAMES + 5 {
        tracker.note_talk_tag_call(format!("Fn{number}").as_bytes(), 1_000);
    }
    assert!(!tracker.start_call(b"Fn0", 1_001).unwrap().from_talk_tag);
    assert!(
        tracker
            .start_call(format!("Fn{}", MAX_TALK_QUEUED_NAMES + 4).as_bytes(), 1_002)
            .unwrap()
            .from_talk_tag
    );
}

#[test]
fn the_talkers_writes_are_the_difference_split_into_message_sized_batches() {
    let before = arrays_with(&[flag(5, true)]);
    let mut after = before.clone();
    after.apply(flag(5, false));
    after.apply(flag(6, true));
    let batches = talker_write_batches(&before, &after);
    assert_eq!(batches, vec![vec![flag(5, false), flag(6, true)]]);
    assert!(talker_write_batches(&after, &after).is_empty());
    let many: Vec<StoryChange> = (0..(MAX_STORY_CHANGES_PER_MESSAGE as u16 + 1))
        .map(|id| flag(id, true))
        .collect();
    let batches = talker_write_batches(&StoryArrays::default(), &arrays_with(&many));
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].len(), MAX_STORY_CHANGES_PER_MESSAGE);
}

#[test]
fn the_host_applies_only_what_its_world_does_not_already_hold() {
    let live = arrays_with(&[flag(5, true)]);
    let outcome = host_write_outcome(
        &live,
        &[flag(5, true), flag(6, true), flag(5, false)],
        &HostChangeLog::new(),
        0,
    );
    assert_eq!(outcome.apply, vec![flag(6, true), flag(5, false)]);
    assert_eq!(outcome.already_held, 1);
}

#[test]
fn a_held_guest_write_is_not_undone_until_the_hold_runs_out() {
    let mut mirror = StoryMirror::default();
    let host = StoryArrays::default();
    for array in StoryArray::ALL {
        mirror.receive_part(&StoryArrayPart {
            array,
            bytes: host.bytes(array).to_vec(),
        });
    }
    mirror.plan(&host, 0);
    let live = arrays_with(&[flag(400, true)]);
    mirror.hold(StoryKey::of(flag(400, true)), 100 + STORY_WRITE_HOLD_MS);
    for now in [100, 2_500, 3_000] {
        assert!(mirror.plan(&live, now).fixes.is_empty(), "at {now}");
    }
    let after = mirror.plan(&live, 100 + STORY_WRITE_HOLD_MS + 5_000);
    assert_eq!(after.fixes, vec![flag(400, false)]);
}

#[test]
fn a_host_change_for_a_held_id_still_lands_at_once() {
    let mut mirror = StoryMirror::default();
    let host = StoryArrays::default();
    for array in StoryArray::ALL {
        mirror.receive_part(&StoryArrayPart {
            array,
            bytes: host.bytes(array).to_vec(),
        });
    }
    mirror.plan(&host, 0);
    mirror.hold(StoryKey::of(flag(400, true)), 3_000);
    mirror.receive_changes(&[flag(400, true)]);
    let plan = mirror.plan(&StoryArrays::default(), 10);
    assert_eq!(plan.fixes, vec![flag(400, true)]);
}

#[test]
fn a_guest_write_for_an_id_the_host_just_changed_is_ignored_until_the_window_passes() {
    let mut log = HostChangeLog::new();
    let host_now = arrays_with(&[flag(5, true)]);
    log.note_feed_changes(&[flag(5, true)], 1_000);
    let outcome = host_write_outcome(&host_now, &[flag(5, false), flag(6, true)], &log, 2_000);
    assert_eq!(outcome.ignored_recent_host_change, 1);
    assert_eq!(outcome.apply, vec![flag(6, true)]);
    let later = host_write_outcome(
        &host_now,
        &[flag(5, false)],
        &log,
        1_000 + HOST_OWN_CHANGE_PRIORITY_MS,
    );
    assert_eq!(later.apply, vec![flag(5, false)]);
}

#[test]
fn a_change_applied_for_the_guest_does_not_count_as_a_host_change() {
    let mut log = HostChangeLog::new();
    log.note_applied_from_guest(flag(7, true), 50);
    log.note_feed_changes(&[flag(7, true)], 100);
    let live = arrays_with(&[flag(7, true)]);
    let outcome = host_write_outcome(&live, &[flag(7, false)], &log, 200);
    assert_eq!(outcome.apply, vec![flag(7, false)]);
    log.note_feed_changes(&[flag(7, true)], 300);
    let again = host_write_outcome(&live, &[flag(7, false)], &log, 400);
    assert_eq!(again.ignored_recent_host_change, 1);
}

#[test]
fn a_talk_baseline_moved_over_the_mirrors_fixes_never_forwards_them() {
    let mut baseline = StoryArrays::default();
    let live = arrays_with(&[flag(9, true), flag(10, true)]);
    rebaseline_over_mirror_fixes(&mut baseline, &[flag(9, true)]);
    let batches = talker_write_batches(&baseline, &live);
    assert_eq!(batches, vec![vec![flag(10, true)]]);
}

#[test]
fn the_end_watch_fires_once_after_the_settle_time() {
    let mut watch = EndWatch::new();
    assert!(!watch.take_if_settled(10_000));
    watch.begin(1_000);
    assert!(!watch.take_if_settled(1_000 + END_WATCH_SETTLE_MS - 1));
    assert!(watch.take_if_settled(1_000 + END_WATCH_SETTLE_MS));
    assert!(!watch.take_if_settled(1_000 + END_WATCH_SETTLE_MS * 2));
}

#[test]
fn a_cleared_end_watch_never_fires() {
    let mut watch = EndWatch::new();
    watch.begin(0);
    watch.clear();
    assert!(!watch.take_if_settled(END_WATCH_SETTLE_MS * 10));
}

fn snow_flag(id: u16, on: bool) -> StoryChange {
    StoryChange::Flag {
        array: StoryArray::SnowFlags,
        id,
        on,
    }
}

fn whole_story(arrays: &StoryArrays) -> Vec<StoryArrayPart> {
    StoryArray::ALL
        .into_iter()
        .map(|array| StoryArrayPart {
            array,
            bytes: arrays.bytes(array).to_vec(),
        })
        .collect()
}

#[test]
fn only_a_connected_guests_own_run_of_a_local_scene_is_watched() {
    let local = SceneScope::of("GM_OUT_Door_of_PopolRoom_13");
    assert!(local.is_local());
    for action in [FireAction::RunLocalSceneOnly, FireAction::RunLocally] {
        assert!(watches_scene_writes(FollowRole::Guest, true, local, action));
        assert!(!watches_scene_writes(FollowRole::Host, true, local, action));
        assert!(!watches_scene_writes(
            FollowRole::Unknown,
            true,
            local,
            action
        ));
        assert!(!watches_scene_writes(
            FollowRole::Guest,
            false,
            local,
            action
        ));
    }
    for led_by_the_host in [
        FireAction::SuppressAndAsk,
        FireAction::DropDuplicate,
        FireAction::RunAndAnnounce,
        FireAction::RunWithoutPartner,
        FireAction::HoldForGatheredPartner,
    ] {
        assert!(!watches_scene_writes(
            FollowRole::Guest,
            true,
            local,
            led_by_the_host
        ));
    }
    for outside_a_local_scene in ["Set_Quest_Step", "EID_0300_a0015g_Start"] {
        let scope = SceneScope::of(outside_a_local_scene);
        assert!(!scope.is_local());
        assert!(!watches_scene_writes(
            FollowRole::Guest,
            true,
            scope,
            FireAction::RunLocally
        ));
    }
}

#[test]
fn a_guests_local_scene_write_reaches_the_host_and_survives_the_hosts_next_state() {
    let mut mirror = StoryMirror::default();
    for part in whole_story(&StoryArrays::default()) {
        mirror.receive_part(&part);
    }
    let before = StoryArrays::default();
    assert!(mirror.plan(&before, 0).fixes.is_empty());

    let mut watch = SceneWriteWatch::new();
    watch.begin("GM_OUT_Door_of_PopolRoom_13", before.clone(), 1_000);
    assert!(!watch.another_call_started(b"GM_OUT_Door_of_PopolRoom_13"));
    let guest_live = arrays_with(&[flag(239, true)]);
    let batches = watch.take_writes(&guest_live, false, 1_016);
    assert_eq!(batches, vec![vec![flag(239, true)]]);
    assert_eq!(watch.watched_scene(), None);
    for &change in &batches[0] {
        mirror.hold(StoryKey::of(change), 1_016 + STORY_WRITE_HOLD_MS);
    }

    let mut host_live = StoryArrays::default();
    let mut host_changes = HostChangeLog::new();
    let outcome = host_write_outcome(&host_live, &batches[0], &host_changes, 1_050);
    assert_eq!(outcome.apply, vec![flag(239, true)]);
    for &change in &outcome.apply {
        host_live.apply(change);
        host_changes.note_applied_from_guest(change, 1_050);
    }
    let mut feed = StoryFeed::default();
    feed.whole(&StoryArrays::default());
    let FeedSend::Changes(sent) = feed.next(&host_live) else {
        panic!("the host sends the guest's write on as its own change");
    };
    assert_eq!(sent[0].changes, vec![flag(239, true)]);

    // The guest keeps its copy while the write is on its way, past the usual drift grace.
    let on_the_way = mirror.plan(&guest_live, 1_016 + STORY_DRIFT_GRACE_MS + 500);
    assert!(on_the_way.fixes.is_empty());
    assert_eq!(on_the_way.held, 1);
    assert!(mirror.receive_changes(&sent[0].changes));
    for later in [4_000, 10_000, 60_000] {
        let plan = mirror.plan(&guest_live, later);
        assert!(plan.fixes.is_empty(), "the guest keeps the flag at {later}");
        assert_eq!(plan.held, 0);
    }
}

#[test]
fn a_local_scene_write_the_host_already_has_changes_nothing_on_the_host() {
    let host_live = arrays_with(&[flag(239, true)]);
    let outcome = host_write_outcome(&host_live, &[flag(239, true)], &HostChangeLog::new(), 50);
    assert!(outcome.apply.is_empty());
    assert_eq!(outcome.already_held, 1);
}

#[test]
fn a_guest_write_outside_a_local_scene_is_still_taken_back_by_the_hosts_state() {
    let mut mirror = StoryMirror::default();
    for part in whole_story(&StoryArrays::default()) {
        mirror.receive_part(&part);
    }
    assert!(mirror.plan(&StoryArrays::default(), 0).fixes.is_empty());
    let mut watch = SceneWriteWatch::new();
    let guest_live = arrays_with(&[flag(239, true)]);
    assert!(watch.take_writes(&guest_live, true, 1_000).is_empty());
    assert!(watch.take_last_writes(&guest_live).is_empty());
    assert!(mirror.plan(&guest_live, 1_000).fixes.is_empty());
    let plan = mirror.plan(&guest_live, 1_000 + STORY_DRIFT_GRACE_MS);
    assert_eq!(plan.fixes, vec![flag(239, false)]);
}

#[test]
fn per_player_and_scratch_writes_of_a_local_scene_stay_on_the_guest() {
    for &id in &TRAP_PRIVATE_GAME_FLAGS {
        assert!(AREA_SCRATCH_GAME_FLAG_IDS.contains(&id));
        assert!(!is_scene_write_for_the_host(flag(id, true)));
    }
    assert!(!is_scene_write_for_the_host(flag(1401, true)));
    assert!(!is_scene_write_for_the_host(flag(1441, true)));
    assert!(!is_scene_write_for_the_host(snow_flag(1075, true)));
    assert!(!is_scene_write_for_the_host(flag(1951, true)));
    assert!(!is_scene_write_for_the_host(flag(239, false)));
    assert!(!is_scene_write_for_the_host(snow_flag(64, false)));
    assert!(is_scene_write_for_the_host(flag(1400, true)));
    assert!(is_scene_write_for_the_host(flag(1455, true)));
    assert!(is_scene_write_for_the_host(snow_flag(64, true)));
    assert!(is_scene_write_for_the_host(StoryChange::Counter {
        array: StoryArray::GameCounters,
        id: 12,
        value: 3,
    }));

    let mut watch = SceneWriteWatch::new();
    watch.begin("GM_IN_Door_of_WestRoom_02", StoryArrays::default(), 0);
    let live = arrays_with(&[flag(1452, true), flag(1432, true), flag(142, true)]);
    assert_eq!(watch.take_last_writes(&live), vec![vec![flag(142, true)]]);
}

#[test]
fn a_scene_that_runs_on_is_watched_until_its_run_ends_and_sends_each_write_once() {
    let mut watch = SceneWriteWatch::new();
    watch.begin("GM_OUT_Door_of_NierHouse_02", StoryArrays::default(), 0);
    assert!(!watch.another_call_started(b"GM_OUT_Door_of_NierHouse_02"));
    let first = arrays_with(&[flag(1599, true)]);
    assert_eq!(
        watch.take_writes(&first, true, 500),
        vec![vec![flag(1599, true)]]
    );
    assert!(watch.take_writes(&first, true, 1_000).is_empty());
    let second = arrays_with(&[flag(1599, true), flag(2029, true)]);
    assert_eq!(
        watch.take_writes(&second, true, 5_000),
        vec![vec![flag(2029, true)]]
    );
    assert!(watch.watched_scene().is_some());
    assert!(watch.another_call_started(b"Set_Quest_Step"));
    assert!(watch.take_last_writes(&second).is_empty());
    assert_eq!(watch.watched_scene(), None);

    watch.begin("GM_OUT_Door_of_NierHouse_02", StoryArrays::default(), 0);
    assert!(
        watch
            .take_writes(&second, true, LOCAL_SCENE_WRITE_WATCH_MAX_MS)
            .len()
            == 1
    );
    assert_eq!(watch.watched_scene(), None);
}

#[test]
fn a_popped_scene_whose_call_has_not_started_is_not_given_up_on_at_once() {
    let mut watch = SceneWriteWatch::new();
    watch.begin("GM_OUT_Door_of_PopolRoom_13", StoryArrays::default(), 0);
    assert!(
        watch
            .take_writes(&StoryArrays::default(), false, 16)
            .is_empty()
    );
    assert!(watch.watched_scene().is_some());
    assert!(!watch.another_call_started(b"GM_OUT_Door_of_PopolRoom_13"));
    let live = arrays_with(&[flag(239, true)]);
    assert_eq!(
        watch.take_writes(&live, false, 32),
        vec![vec![flag(239, true)]]
    );
    assert_eq!(watch.watched_scene(), None);

    watch.begin("GM_OUT_Door_of_PopolRoom_13", StoryArrays::default(), 0);
    watch.take_writes(
        &StoryArrays::default(),
        false,
        LOCAL_SCENE_CALL_START_WAIT_MAX_MS,
    );
    assert_eq!(watch.watched_scene(), None);
}

#[test]
fn a_change_the_hosts_state_made_during_a_local_scene_is_not_sent_back() {
    let mut watch = SceneWriteWatch::new();
    watch.begin("GM_OUT_Door_of_PopolRoom_13", StoryArrays::default(), 0);
    watch.rebaseline_over_mirror_fixes(&[flag(137, true)]);
    let live = arrays_with(&[flag(137, true), flag(239, true)]);
    assert_eq!(watch.take_last_writes(&live), vec![vec![flag(239, true)]]);
}
