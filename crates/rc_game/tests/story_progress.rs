use rc_game::story_progress::{
    FeedSend, GAME_FLAGS_BLOCK_OFFSET, PLAYER_NAME_BLOCK_OFFSET, PLAYER_NAME_BYTES,
    RANK_SIDECAR_BYTES, SAVE_FILE_BYTES, SLOT_BYTES, SLOT_CHECKSUM_OFFSET, SLOTS_FILE_OFFSET,
    SNOW_COUNTERS_BLOCK_OFFSET, STORY_CHANGES_BEFORE_A_FULL_RESEND, STORY_DRIFT_GRACE_MS,
    STORY_DRIFT_KEYS_THAT_MEAN_A_RELOAD, SlotOutcome, SlotStory, StoryArrays, StoryFeed,
    StoryMirror, StoryRank, StoryRule, StorySource, apply_story_rule, decode_rank_sidecar,
    encode_rank_sidecar, milestone_ids, part_of_phase, slot_checksum,
};
use rc_proto::{
    MAX_STORY_CHANGES_PER_MESSAGE, STORY_ARRAY_BYTES, StoryArray, StoryArrayPart, StoryChange,
};

const PLAY_TIME_SLOT_OFFSET: usize = 0x4A0;
const GOLD_SLOT_OFFSET: usize = 0xBC;

fn flag(array: StoryArray, id: u16, on: bool) -> StoryChange {
    StoryChange::Flag { array, id, on }
}

fn arrays_with(changes: &[StoryChange]) -> StoryArrays {
    let mut arrays = StoryArrays::default();
    for &change in changes {
        arrays.apply(change);
    }
    arrays
}

fn milestone_arrays(count: usize) -> StoryArrays {
    let changes: Vec<StoryChange> = milestone_ids(StoryArray::GameFlags)
        .iter()
        .take(count)
        .map(|&id| flag(StoryArray::GameFlags, id, true))
        .collect();
    arrays_with(&changes)
}

fn story(phase: &str, milestones: usize) -> SlotStory {
    let mut slot = vec![0_u8; SLOT_BYTES];
    slot[4..4 + phase.len()].copy_from_slice(phase.as_bytes());
    slot[0x24] = 13;
    milestone_arrays(milestones).write_to_block(&mut slot);
    SlotStory::read(&slot).expect("a whole slot")
}

fn name(text: &str) -> [u8; PLAYER_NAME_BYTES] {
    let mut bytes = [0_u8; PLAYER_NAME_BYTES];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    bytes
}

fn whole_parts(arrays: &StoryArrays) -> Vec<StoryArrayPart> {
    StoryArray::ALL
        .into_iter()
        .map(|array| StoryArrayPart {
            array,
            bytes: arrays.bytes(array).to_vec(),
        })
        .collect()
}

fn slot_range(index: usize) -> std::ops::Range<usize> {
    let start = SLOTS_FILE_OFFSET + index * SLOT_BYTES;
    start..start + SLOT_BYTES
}

fn write_slot(file: &mut [u8], index: usize, player: &str, story: &SlotStory, play_time: f64) {
    let slot = &mut file[slot_range(index)];
    slot[..4].copy_from_slice(&200_u32.to_le_bytes());
    slot[PLAYER_NAME_BLOCK_OFFSET..PLAYER_NAME_BLOCK_OFFSET + PLAYER_NAME_BYTES]
        .copy_from_slice(&name(player));
    slot[PLAY_TIME_SLOT_OFFSET..PLAY_TIME_SLOT_OFFSET + 8]
        .copy_from_slice(&play_time.to_le_bytes());
    story.write(slot);
    rc_game::story_progress::fix_slot_checksum(slot);
}

fn slot_story(file: &[u8], index: usize) -> SlotStory {
    SlotStory::read(&file[slot_range(index)]).expect("a whole slot")
}

fn written_source(outcomes: &[SlotOutcome]) -> Option<StorySource> {
    outcomes.iter().find_map(|outcome| match outcome {
        SlotOutcome::Written { source, .. } => Some(*source),
        _ => None,
    })
}

#[test]
fn the_milestone_lists_hold_the_counted_ids_once_each_in_order() {
    let game = milestone_ids(StoryArray::GameFlags);
    let snow = milestone_ids(StoryArray::SnowFlags);
    assert_eq!(game.len(), 522);
    assert_eq!(snow.len(), 53);
    for ids in [game, snow] {
        assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));
    }
    assert!(milestone_ids(StoryArray::GameCounters).is_empty());
}

#[test]
fn flag_ids_use_the_games_bit_layout_at_the_block_offsets() {
    let arrays = arrays_with(&[flag(StoryArray::GameFlags, 70, true)]);
    assert_eq!(arrays.bytes(StoryArray::GameFlags)[8], 1 << 6);
    let mut block = vec![0_u8; 0xE30];
    assert!(arrays.write_to_block(&mut block));
    assert_eq!(block[GAME_FLAGS_BLOCK_OFFSET + 8], 1 << 6);
    assert_eq!(StoryArrays::from_block(&block), Some(arrays));
    assert_eq!(
        StoryArrays::from_block(&block[..SNOW_COUNTERS_BLOCK_OFFSET]),
        None
    );
}

#[test]
fn changes_clear_first_then_set_then_store_counters_and_reach_the_target() {
    let from = arrays_with(&[
        flag(StoryArray::GameFlags, 5, true),
        flag(StoryArray::SnowFlags, 900, true),
    ]);
    let target = arrays_with(&[
        flag(StoryArray::GameFlags, 6, true),
        StoryChange::Counter {
            array: StoryArray::GameCounters,
            id: 3,
            value: 9,
        },
    ]);
    let changes = from.changes_to(&target);
    assert_eq!(
        changes,
        vec![
            flag(StoryArray::GameFlags, 5, false),
            flag(StoryArray::SnowFlags, 900, false),
            flag(StoryArray::GameFlags, 6, true),
            StoryChange::Counter {
                array: StoryArray::GameCounters,
                id: 3,
                value: 9,
            },
        ]
    );
    let mut reached = from.clone();
    for change in changes {
        reached.apply(change);
    }
    assert_eq!(reached, target);
    assert_eq!(reached.digest(), target.digest());
    assert_ne!(from.digest(), target.digest());
}

#[test]
fn the_host_feed_sends_nothing_until_the_guest_has_the_whole_state() {
    let mut feed = StoryFeed::default();
    let current = arrays_with(&[flag(StoryArray::GameFlags, 1, true)]);
    assert_eq!(feed.next(&current), FeedSend::Nothing);
    assert_eq!(feed.sent_digest(), None);
    let parts = feed.whole(&current);
    assert_eq!(parts.len(), 4);
    assert!(
        parts
            .iter()
            .all(|part| part.bytes.len() == STORY_ARRAY_BYTES)
    );
    assert_eq!(feed.sent_digest(), Some(current.digest()));
    assert_eq!(feed.next(&current), FeedSend::Nothing);
}

#[test]
fn the_host_feed_batches_changes_and_resends_whole_arrays_after_a_reset() {
    let mut feed = StoryFeed::default();
    let start = StoryArrays::default();
    feed.whole(&start);
    let many: Vec<StoryChange> = (0..u16::try_from(MAX_STORY_CHANGES_PER_MESSAGE + 1).unwrap())
        .map(|id| flag(StoryArray::GameFlags, id, true))
        .collect();
    let after = arrays_with(&many);
    let FeedSend::Changes(batches) = feed.next(&after) else {
        panic!("expected changes");
    };
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].changes.len(), MAX_STORY_CHANGES_PER_MESSAGE);
    let reset: Vec<StoryChange> = (0..u16::try_from(STORY_CHANGES_BEFORE_A_FULL_RESEND + 1)
        .unwrap())
        .map(|id| flag(StoryArray::SnowFlags, id, true))
        .collect();
    let mut after_reset = after.clone();
    for change in reset {
        after_reset.apply(change);
    }
    assert!(matches!(feed.next(&after_reset), FeedSend::WholeArrays(parts) if parts.len() == 4));
    assert_eq!(feed.sent_digest(), Some(after_reset.digest()));
}

#[test]
fn the_mirror_waits_for_all_four_arrays_and_ignores_changes_before() {
    let mut mirror = StoryMirror::default();
    let host = arrays_with(&[flag(StoryArray::GameFlags, 9, true)]);
    let parts = whole_parts(&host);
    for part in &parts[..3] {
        assert!(mirror.receive_part(part));
    }
    assert!(!mirror.receive_changes(&[flag(StoryArray::GameFlags, 10, true)]));
    assert!(mirror.plan(&StoryArrays::default(), 0).fixes.is_empty());
    assert_eq!(mirror.host_digest(), None);
    assert!(mirror.receive_part(&parts[3]));
    assert_eq!(mirror.host_digest(), Some(host.digest()));
}

#[test]
fn the_host_state_is_applied_at_once_and_a_guest_only_flag_waits_for_the_grace() {
    let mut mirror = StoryMirror::default();
    let host = arrays_with(&[flag(StoryArray::GameFlags, 9, true)]);
    for part in whole_parts(&host) {
        mirror.receive_part(&part);
    }
    let live = StoryArrays::default();
    let plan = mirror.plan(&live, 1_000);
    assert_eq!(plan.fixes, vec![flag(StoryArray::GameFlags, 9, true)]);
    let mut live = host.clone();
    live.apply(flag(StoryArray::GameFlags, 400, true));
    let held = mirror.plan(&live, 2_000);
    assert!(held.fixes.is_empty());
    assert_eq!(held.held, 1);
    let due = mirror.plan(&live, 2_000 + STORY_DRIFT_GRACE_MS);
    assert_eq!(due.fixes, vec![flag(StoryArray::GameFlags, 400, false)]);
    mirror.receive_changes(&[flag(StoryArray::GameFlags, 401, true)]);
    let host_change = mirror.plan(&host, 2_100 + STORY_DRIFT_GRACE_MS);
    assert_eq!(
        host_change.fixes,
        vec![flag(StoryArray::GameFlags, 401, true)]
    );
}

#[test]
fn a_reload_that_replaces_the_story_is_put_back_at_once() {
    let mut mirror = StoryMirror::default();
    let host = arrays_with(
        &(0..u16::try_from(STORY_DRIFT_KEYS_THAT_MEAN_A_RELOAD + 1).unwrap())
            .map(|id| flag(StoryArray::GameFlags, id, true))
            .collect::<Vec<_>>(),
    );
    for part in whole_parts(&host) {
        mirror.receive_part(&part);
    }
    assert_eq!(mirror.plan(&host, 0).fixes.len(), 0);
    let plan = mirror.plan(&StoryArrays::default(), 10);
    assert!(plan.reload_restored);
    assert_eq!(plan.fixes.len(), STORY_DRIFT_KEYS_THAT_MEAN_A_RELOAD + 1);
}

#[test]
fn the_part_letter_ranks_before_the_milestone_count() {
    let early_part_far_on = StoryRank {
        part: part_of_phase(b"A_FIELD"),
        milestones: 400,
    };
    let later_part_new = StoryRank {
        part: part_of_phase(b"B_FIELD"),
        milestones: 10,
    };
    assert!(later_part_new.outranks(&early_part_far_on));
    let unknown_letter = StoryRank {
        part: part_of_phase(b"X_ROOM"),
        milestones: 11,
    };
    assert_eq!(unknown_letter.part, None);
    assert!(unknown_letter.outranks(&later_part_new));
    assert!(!later_part_new.outranks(&later_part_new));
    assert_eq!(part_of_phase(b"E_END"), Some(5));
    assert_eq!(part_of_phase(b""), None);
}

#[test]
fn the_slot_checksum_is_the_byte_sum_of_the_first_0xc20_bytes() {
    let mut slot = vec![0_u8; SLOT_BYTES];
    slot[0] = 200;
    slot[0xC1F] = 5;
    slot[0xC20] = 99;
    assert_eq!(slot_checksum(&slot), Some(205));
}

#[test]
fn a_brand_new_guest_save_takes_the_live_story() {
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    let on_disk = file.clone();
    write_slot(&mut file, 0, "NIER", &story("A_HOST", 40), 10.0);
    let mut rule = StoryRule::new(story("A_START", 0), name("NIER"), None);
    let outcomes = apply_story_rule(&mut file, Some(&on_disk), &mut rule);
    assert_eq!(written_source(&outcomes), Some(StorySource::Live));
    assert_eq!(slot_story(&file, 0), story("A_HOST", 40));
    assert_eq!(rule.high_mark.map(|mark| mark.milestones), Some(40));
}

#[test]
fn a_guest_far_ahead_keeps_its_own_story_with_its_live_personal_data() {
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    let on_disk = file.clone();
    write_slot(&mut file, 2, "NIER", &story("A_HOST", 3), 50.0);
    file[slot_range(2)][GOLD_SLOT_OFFSET] = 77;
    rc_game::story_progress::fix_slot_checksum(&mut file[slot_range(2)]);
    let own = story("B_OWN", 30);
    let mut rule = StoryRule::new(own.clone(), name("NIER"), None);
    let outcomes = apply_story_rule(&mut file, Some(&on_disk), &mut rule);
    assert_eq!(written_source(&outcomes), Some(StorySource::Own));
    let slot = &file[slot_range(2)];
    assert_eq!(SlotStory::read(slot), Some(own));
    assert_eq!(slot[GOLD_SLOT_OFFSET], 77);
    let stored = u32::from_le_bytes(
        slot[SLOT_CHECKSUM_OFFSET..SLOT_CHECKSUM_OFFSET + 4]
            .try_into()
            .unwrap(),
    );
    assert_eq!(Some(stored), slot_checksum(slot));
}

#[test]
fn a_tie_goes_to_the_live_story() {
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    let on_disk = file.clone();
    let live = story("A_HOST", 12);
    write_slot(&mut file, 0, "NIER", &live, 10.0);
    let mut rule = StoryRule::new(story("A_OWN", 12), name("NIER"), None);
    let outcomes = apply_story_rule(&mut file, Some(&on_disk), &mut rule);
    assert_eq!(written_source(&outcomes), Some(StorySource::Live));
    assert_eq!(slot_story(&file, 0), live);
}

#[test]
fn a_host_reset_does_not_lower_what_this_session_already_saved() {
    let mut rule = StoryRule::new(story("A_OWN", 0), name("NIER"), None);
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    let empty_disk = file.clone();
    write_slot(&mut file, 0, "NIER", &story("A_HOST", 60), 10.0);
    apply_story_rule(&mut file, Some(&empty_disk), &mut rule);
    let on_disk = file.clone();
    write_slot(&mut file, 0, "NIER", &story("A_HOST", 2), 20.0);
    let outcomes = apply_story_rule(&mut file, Some(&on_disk), &mut rule);
    assert_eq!(written_source(&outcomes), Some(StorySource::SessionBest));
    assert_eq!(slot_story(&file, 0), story("A_HOST", 60));
}

#[test]
fn a_mark_from_an_earlier_session_is_reported_when_the_save_falls_below_it() {
    let mark = StoryRank {
        part: Some(2),
        milestones: 90,
    };
    let mut rule = StoryRule::new(story("A_OWN", 0), name("NIER"), Some(mark));
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    let on_disk = file.clone();
    write_slot(&mut file, 0, "NIER", &story("A_HOST", 5), 10.0);
    let outcomes = apply_story_rule(&mut file, Some(&on_disk), &mut rule);
    assert!(matches!(
        outcomes[0],
        SlotOutcome::Written {
            below_high_mark: true,
            ..
        }
    ));
    assert_eq!(rule.high_mark, Some(mark));
}

#[test]
fn unchanged_slots_and_other_characters_are_left_alone() {
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    write_slot(&mut file, 0, "NIER", &story("B_OLD", 80), 10.0);
    let on_disk = file.clone();
    write_slot(&mut file, 1, "OTHER", &story("A_HOST", 5), 20.0);
    let mut rule = StoryRule::new(story("C_OWN", 300), name("NIER"), None);
    let outcomes = apply_story_rule(&mut file, Some(&on_disk), &mut rule);
    assert_eq!(outcomes, vec![SlotOutcome::OtherCharacter { slot: 1 }]);
    assert_eq!(slot_story(&file, 1), story("A_HOST", 5));
    assert_eq!(&file[slot_range(0)], &on_disk[slot_range(0)]);
}

#[test]
fn a_slot_that_differs_only_in_the_story_written_before_is_kept_as_on_disk() {
    let mut rule = StoryRule::new(story("B_OWN", 30), name("NIER"), None);
    let mut file = vec![0_u8; SAVE_FILE_BYTES];
    let empty_disk = file.clone();
    write_slot(&mut file, 0, "NIER", &story("A_HOST", 3), 10.0);
    let image = file.clone();
    apply_story_rule(&mut file, Some(&empty_disk), &mut rule);
    let on_disk = file.clone();
    let mut next = image.clone();
    write_slot(&mut next, 1, "NIER", &story("A_HOST", 4), 30.0);
    let outcomes = apply_story_rule(&mut next, Some(&on_disk), &mut rule);
    assert!(outcomes.contains(&SlotOutcome::KeptAsOnDisk { slot: 0 }));
    assert_eq!(&next[slot_range(0)], &on_disk[slot_range(0)]);
}

#[test]
fn the_rank_sidecar_round_trips_and_a_damaged_one_means_no_mark() {
    let rank = StoryRank {
        part: Some(3),
        milestones: 412,
    };
    let bytes = encode_rank_sidecar(rank);
    assert_eq!(bytes.len(), RANK_SIDECAR_BYTES);
    assert_eq!(decode_rank_sidecar(&bytes), Some(rank));
    let no_part = StoryRank {
        part: None,
        milestones: 1,
    };
    assert_eq!(
        decode_rank_sidecar(&encode_rank_sidecar(no_part)),
        Some(no_part)
    );
    let mut damaged = bytes;
    damaged[0] ^= 1;
    assert_eq!(decode_rank_sidecar(&damaged), None);
    assert_eq!(decode_rank_sidecar(&bytes[..8]), None);
}
