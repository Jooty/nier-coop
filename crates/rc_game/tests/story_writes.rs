use rc_game::story_calls::name_hash;
use rc_game::story_progress::{StoryArrays, StoryKey, StoryMirror};
use rc_game::story_writes::{
    HOST_OWN_CHANGE_PRIORITY_MS, HostChangeLog, MAX_TALK_QUEUED_NAMES, STORY_WRITE_HOLD_MS,
    ScriptCallTracker, TALK_CALLBACK_ATTRIBUTION_MAX_MS, TALK_QUEUED_NAME_LIFETIME_MS,
    host_write_outcome, rebaseline_over_mirror_fixes, talk_tag_function_name, talker_write_batches,
};
use rc_proto::{MAX_STORY_CHANGES_PER_MESSAGE, StoryArray, StoryArrayPart, StoryChange};

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
