use rc_game::script_rolls::{
    RollFacts, RollNumber, RollOrdinals, SHARED_ROLL_EXPIRY_MS, SHARED_ROLL_STORE_MAX_ENTRIES,
    SharedRollStore, number_from_slot, roll_is_shared, value_fits, value_fits_arguments,
};
use rc_proto::{MAX_SCRIPT_ROLL_ORDINAL, ScriptRoll, ScriptRollValue};

fn roll(phase: u16, call: u64, ordinal: u16, value: i32) -> ScriptRoll {
    ScriptRoll {
        phase,
        call,
        ordinal,
        value: ScriptRollValue::Int(value),
    }
}

const COUNTER_FUNCTION_HASH: u64 = 0xF942_280A_1514_6DF7;
const LOAD_FUNCTION_HASH: u64 = 0xCE0E_ECAD_70F2_71E9;
const LOAD_FUNCTION_PACK_HASH: u64 = 0x7FC3_DDA4_3F9E_64CE;
const PLACEMENT_FUNCTION_HASH: u64 = 0xCE57_6E6B_8F0A_F230;
const GATHERING_FUNCTION_HASHES: [u64; 3] = [
    0x420F_FD1C_A89C_FF66,
    0xFAD1_4D85_9A87_9F54,
    0x3998_1B93_BB99_C492,
];

fn facts(function_hash: u64) -> RollFacts {
    RollFacts {
        function_hash,
        phase_hash: Some(0xC979_8DA6_FC23_E7D1),
        relayed_story_call: false,
        from_talk_tag: false,
    }
}

#[test]
fn an_ordinal_advances_only_when_committed_and_restarts_with_the_next_run() {
    let mut ordinals = RollOrdinals::new();
    assert_eq!(ordinals.peek(5, 100), 0);
    assert_eq!(ordinals.peek(5, 100), 0);
    ordinals.commit(5, 100);
    assert_eq!(ordinals.peek(5, 100), 1);
    ordinals.commit(5, 100);
    assert_eq!(ordinals.peek(5, 100), 2);
    assert_eq!(ordinals.peek(5, 250), 0);
    assert_eq!(ordinals.peek(6, 250), 0);
}

#[test]
fn interleaved_calls_keep_their_own_counters() {
    let mut ordinals = RollOrdinals::new();
    let mut seen = Vec::new();
    for (call, started) in [(1, 10), (2, 20), (1, 10), (2, 20), (1, 10)] {
        seen.push((call, ordinals.peek(call, started)));
        ordinals.commit(call, started);
    }
    assert_eq!(seen, vec![(1, 0), (2, 0), (1, 1), (2, 1), (1, 2)]);
}

#[test]
fn a_failed_read_leaves_the_counter_alone() {
    let mut ordinals = RollOrdinals::new();
    ordinals.commit(1, 1);
    assert_eq!(ordinals.peek(1, 1), 1);
    assert_eq!(ordinals.peek(1, 1), 1);
}

#[test]
fn ordinals_stop_at_the_wire_limit() {
    let mut ordinals = RollOrdinals::new();
    for _ in 0..MAX_SCRIPT_ROLL_ORDINAL + 20 {
        ordinals.commit(1, 1);
    }
    assert_eq!(ordinals.peek(1, 1), MAX_SCRIPT_ROLL_ORDINAL);
}

#[test]
fn bounds_come_from_the_arguments_and_a_float_without_bounds_stays_in_zero_to_one() {
    let low = Some(RollNumber(1.0));
    let high = Some(RollNumber(6.0));
    assert!(value_fits_arguments(ScriptRollValue::Int(6), 2, low, high));
    assert!(!value_fits_arguments(ScriptRollValue::Int(7), 2, low, high));
    assert!(!value_fits_arguments(
        ScriptRollValue::Int(3),
        2,
        None,
        high
    ));
    assert!(value_fits_arguments(
        ScriptRollValue::Float(0.5),
        0,
        None,
        None
    ));
    assert!(value_fits_arguments(
        ScriptRollValue::Float(0.0),
        1,
        low,
        high
    ));
    assert!(!value_fits_arguments(
        ScriptRollValue::Float(1.0),
        1,
        low,
        high
    ));
    assert!(!value_fits_arguments(ScriptRollValue::Int(1), 1, low, high));
}

#[test]
fn a_stored_roll_is_used_once_by_its_exact_key() {
    let mut store = SharedRollStore::new();
    store.offer(roll(3, 9, 0, 11), 1_000);
    assert_eq!(store.take(3, 9, 1, 1_001), None);
    assert_eq!(store.take(4, 9, 0, 1_001), None);
    assert_eq!(store.take(3, 8, 0, 1_001), None);
    assert_eq!(store.take(3, 9, 0, 1_001), Some(ScriptRollValue::Int(11)));
    assert_eq!(store.take(3, 9, 0, 1_002), None);
}

#[test]
fn two_runs_of_one_call_are_used_oldest_first() {
    let mut store = SharedRollStore::new();
    store.offer(roll(3, 9, 0, 1), 1_000);
    store.offer(roll(3, 9, 0, 2), 1_100);
    assert_eq!(store.take(3, 9, 0, 1_200), Some(ScriptRollValue::Int(1)));
    assert_eq!(store.take(3, 9, 0, 1_200), Some(ScriptRollValue::Int(2)));
}

#[test]
fn a_roll_expires_after_the_expiry_window() {
    let mut store = SharedRollStore::new();
    store.offer(roll(3, 9, 0, 11), 1_000);
    assert_eq!(store.take(3, 9, 0, 1_000 + SHARED_ROLL_EXPIRY_MS), None);
    assert!(store.is_empty());
}

#[test]
fn a_phase_change_forgets_the_old_phase_rolls() {
    let mut store = SharedRollStore::new();
    store.offer(roll(3, 9, 0, 1), 1_000);
    store.offer(roll(4, 9, 0, 2), 1_000);
    assert_eq!(store.forget_other_phases(4), 1);
    assert_eq!(store.len(), 1);
    assert_eq!(store.take(4, 9, 0, 1_001), Some(ScriptRollValue::Int(2)));
}

#[test]
fn the_store_drops_its_oldest_roll_when_full() {
    let mut store = SharedRollStore::new();
    for ordinal in 0..=u16::try_from(SHARED_ROLL_STORE_MAX_ENTRIES).unwrap() {
        store.offer(roll(1, 1, ordinal, 0), 1_000);
    }
    assert_eq!(store.len(), SHARED_ROLL_STORE_MAX_ENTRIES);
    assert_eq!(store.take(1, 1, 0, 1_001), None);
}

#[test]
fn a_value_outside_the_guests_own_bounds_is_a_miss() {
    let (low, high) = (RollNumber(1.0), RollNumber(6.0));
    assert!(value_fits(ScriptRollValue::Int(6), low, high));
    assert!(!value_fits(ScriptRollValue::Int(7), low, high));
    assert!(value_fits(
        ScriptRollValue::Float(0.5),
        RollNumber(0.0),
        RollNumber(1.0)
    ));
    assert!(!value_fits(
        ScriptRollValue::Float(1.5),
        RollNumber(0.0),
        RollNumber(1.0)
    ));
    assert!(value_fits(ScriptRollValue::Int(3), high, low));
}

#[test]
fn script_numbers_decode_by_their_tag() {
    let int_payload = 9_i32.to_le_bytes();
    let padded = [
        int_payload[0],
        int_payload[1],
        int_payload[2],
        int_payload[3],
        0,
        0,
        0,
        0,
    ];
    assert_eq!(number_from_slot(4, padded), Some(RollNumber(9.0)));
    assert_eq!(
        number_from_slot(1, 2.5_f64.to_le_bytes()),
        Some(RollNumber(2.5))
    );
    assert_eq!(number_from_slot(2, padded), None);
}

#[test]
fn a_relayed_story_call_shares_its_rolls() {
    let mut relayed = facts(0x1234);
    assert!(!roll_is_shared(&relayed));
    relayed.relayed_story_call = true;
    assert!(roll_is_shared(&relayed));
}

#[test]
fn a_talk_tag_call_never_shares_its_rolls() {
    let mut talk = facts(COUNTER_FUNCTION_HASH);
    talk.relayed_story_call = true;
    talk.from_talk_tag = true;
    assert!(!roll_is_shared(&talk));
}

#[test]
fn allow_listed_load_functions_share_their_rolls() {
    assert!(roll_is_shared(&facts(COUNTER_FUNCTION_HASH)));
    assert!(roll_is_shared(&facts(PLACEMENT_FUNCTION_HASH)));
}

#[test]
fn a_phase_bound_allow_entry_needs_that_phase() {
    let mut load = facts(LOAD_FUNCTION_HASH);
    assert!(!roll_is_shared(&load));
    load.phase_hash = Some(LOAD_FUNCTION_PACK_HASH);
    assert!(roll_is_shared(&load));
}

#[test]
fn gathering_points_keep_their_own_rolls_even_in_a_relayed_call() {
    for hash in GATHERING_FUNCTION_HASHES {
        let mut gathering = facts(hash);
        gathering.relayed_story_call = true;
        assert!(!roll_is_shared(&gathering), "{hash:016x}");
    }
}
