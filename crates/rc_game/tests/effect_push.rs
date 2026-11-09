use rc_game::effects::{
    AFTER_OWN_HIT_WINDOW_MS, DROP_LOG_INTERVAL_MS, DropLogLimiter, FLUSHER_HIT_RADIUS,
    MAX_RECENT_PUSHED_HANDLES, PendingAdmission, PushCaller, RecentHandles, admit_pending,
    census_log_line, classify_push_caller, push_is_sent, read_request_name, read_request_transform,
    within_after_hit_window, within_hit_radius,
};
use rc_game::offsets::{
    EFFECT_NAME_ENTRY_FIRST_OFFSET, EFFECT_NAME_ENTRY_ID_OFFSET, EFFECT_NAME_ENTRY_STRIDE,
    EFFECT_NAME_TABLES_OFFSET, EFFECT_REGISTRY_RVA,
};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const REQUEST: u64 = 0x2_3000_0000;
const TABLE: u64 = 0x2_4000_0000;
const NAME: u64 = 0x2_5000_0000;
const INDEX: u32 = 5;
const GENERATION: u32 = 2;

fn memory_with_request(id_in_entry: u32) -> FakeMemory {
    let mut memory = FakeMemory::new();
    memory.put(REQUEST, &INDEX.to_le_bytes());
    memory.put(REQUEST + 4, &GENERATION.to_le_bytes());
    let registry = FAKE_MODULE_BASE + u64::from(EFFECT_REGISTRY_RVA);
    memory.put(
        registry + EFFECT_NAME_TABLES_OFFSET + u64::from(INDEX) * 8,
        &TABLE.to_le_bytes(),
    );
    memory.put(TABLE, &4_i32.to_le_bytes());
    let entry =
        TABLE + EFFECT_NAME_ENTRY_FIRST_OFFSET + u64::from(GENERATION) * EFFECT_NAME_ENTRY_STRIDE;
    memory.put(entry, &NAME.to_le_bytes());
    memory.put(
        entry + EFFECT_NAME_ENTRY_ID_OFFSET,
        &id_in_entry.to_le_bytes(),
    );
    memory.put(NAME, b"hitSword01\0");
    memory
}

#[test]
fn a_request_names_its_effect_through_the_registry() {
    let memory = memory_with_request((GENERATION << 16) | INDEX);
    assert_eq!(
        read_request_name(&memory, REQUEST).unwrap().as_deref(),
        Some("hitSword01")
    );
}

#[test]
fn a_registry_entry_for_another_id_gives_no_name() {
    let memory = memory_with_request(999);
    assert_eq!(read_request_name(&memory, REQUEST).unwrap(), None);
}

#[test]
fn a_request_reads_its_matrix() {
    let mut memory = FakeMemory::new();
    for index in 0..12_u64 {
        memory.put(REQUEST + 8 + index * 4, &(index as f32).to_le_bytes());
    }
    let matrix = read_request_transform(&memory, REQUEST).unwrap();
    assert_eq!(matrix[3], 3.0);
    assert_eq!(matrix[11], 11.0);
}

#[test]
fn callers_are_told_apart_by_their_function() {
    assert_eq!(classify_push_caller(0x35_7AA0), PushCaller::ByName);
    assert_eq!(classify_push_caller(0x35_6D00), PushCaller::FlusherA);
    assert_eq!(classify_push_caller(0x35_7EF5), PushCaller::FlusherB);
    assert_eq!(classify_push_caller(0x10_0000), PushCaller::Other);
}

#[test]
fn only_flusher_spawns_near_a_hit_are_sent_by_the_push() {
    let late = Some(AFTER_OWN_HIT_WINDOW_MS + 1);
    assert!(push_is_sent(
        PushCaller::FlusherA,
        false,
        Some(AFTER_OWN_HIT_WINDOW_MS),
        true
    ));
    assert!(push_is_sent(PushCaller::FlusherB, true, None, true));
    assert!(!push_is_sent(PushCaller::FlusherA, false, late, true));
    assert!(!push_is_sent(PushCaller::FlusherA, false, None, true));
    assert!(!push_is_sent(PushCaller::ByName, true, Some(0), true));
    assert!(within_after_hit_window(Some(0)));
    assert!(!within_after_hit_window(None));
}

#[test]
fn a_handle_is_pushed_once_while_remembered() {
    let mut recent = RecentHandles::default();
    assert!(recent.first_time(7));
    assert!(!recent.first_time(7));
    for handle in 100..100 + MAX_RECENT_PUSHED_HANDLES as u32 {
        assert!(recent.first_time(handle));
    }
    assert!(recent.first_time(7));
}

#[test]
fn the_census_line_names_effect_caller_and_hit_timing() {
    assert_eq!(
        census_log_line(
            "hitBlood02",
            PushCaller::FlusherA,
            0x35_6D00,
            Some(40),
            false
        ),
        "effects: effect census hitBlood02 from flusher-a 0x356d00 hit_ms=40 in_flight=false"
    );
    assert!(census_log_line("x", PushCaller::Other, 1, None, true).contains("hit_ms=none"));
}

#[test]
fn a_flusher_spawn_far_from_the_last_hit_is_not_sent() {
    let hit = Some([10.0, 0.0, 5.0]);
    assert!(within_hit_radius([10.0, 0.0, 5.0], hit));
    assert!(within_hit_radius([10.0 + FLUSHER_HIT_RADIUS, 0.0, 5.0], hit));
    assert!(!within_hit_radius([10.0 + FLUSHER_HIT_RADIUS + 0.5, 0.0, 5.0], hit));
    assert!(!within_hit_radius([10.0, 0.0, 5.0], None));
    assert!(!push_is_sent(PushCaller::FlusherA, true, Some(0), false));
}

#[test]
fn a_full_queue_makes_room_for_in_hit_effects_by_dropping_flusher_ones() {
    assert_eq!(admit_pending(&[true, false], false, 3), PendingAdmission::Append);
    assert_eq!(admit_pending(&[true, false, false], false, 3), PendingAdmission::Drop);
    assert_eq!(
        admit_pending(&[false, true, false], true, 3),
        PendingAdmission::ReplaceAt(2)
    );
    assert_eq!(admit_pending(&[true, true, true], true, 3), PendingAdmission::Drop);
}

#[test]
fn a_drop_is_logged_once_per_second() {
    let mut limiter = DropLogLimiter::default();
    assert!(limiter.should_log(5000));
    assert!(!limiter.should_log(5000 + DROP_LOG_INTERVAL_MS - 1));
    assert!(limiter.should_log(5000 + DROP_LOG_INTERVAL_MS));
}
