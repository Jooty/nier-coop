use rc_game::bullet_flights::{
    BULLET_FLIGHT_STALE_MS, BULLET_LOOK_REPEAT_EVERY_MESSAGES, BULLET_RENDER_DELAY_MS,
    BulletFlightTable, BulletTracker, FlightSample, MAX_EXTRAPOLATION_MS,
    MAX_PARTNER_BULLET_FLIGHTS, MAX_SIGHTED_BULLETS, SightedBullet, bullets_of_player,
    partner_effect_id, smoothed_transform, style_is_set,
};
use rc_game::offsets::{
    ACTOR_SELF_HANDLE_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET,
    BOOK_ACTOR_POINTER_RVA, EFFECT_INSTANCE_STYLE_OFFSET, EFFECT_POOL_GENERATION_OFFSET,
    EFFECT_POOL_RVA, EFFECT_POOL_SLOT_STRIDE, HANDLE_POOL_GENERATIONS_RVA,
    HANDLE_POOL_POINTERS_RVA, HANDLE_POOL_SLOT_COUNT, NORMAL_BULLET_EFFECT_HANDLES_OFFSET,
    NORMAL_BULLET_HIT_ID_OFFSET, NORMAL_BULLET_OWNER_HANDLE_OFFSET,
    NORMAL_BULLET_RECORD_EFFECT_IDS_OFFSET, NORMAL_BULLET_RECORD_POINTER_OFFSET,
    NORMAL_BULLET_STATE_OFFSET, NORMAL_BULLET_VTABLE_RVA,
};
use rc_proto::{BULLET_STYLE_FLOATS, BulletLook, MAX_BULLET_EFFECT_ID};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const PLAYER: u64 = 0x2_0000_0000;
const ENEMY: u64 = 0x2_3000_0000;
const BULLET_HEAP: u64 = 0x2_4000_0000;
const BULLET_STRIDE: u64 = 0x2_0000;
const RECORD_HEAP: u64 = 0x2_6000_0000;
const EFFECT_HEAP: u64 = 0x2_8000_0000;
const PLAYER_SLOT: u32 = 5;
const ENEMY_SLOT: u32 = 6;
const FIRST_BULLET_SLOT: u32 = 100;
const GENERATION: u32 = 9;
const EFFECT_GENERATION: i32 = 3;
const EFFECT_IDS: [i32; 4] = [0x2710, 0x2711, 0, 0];

fn pool_entry(memory: &mut FakeMemory, slot: u32, actor: u64, generation: u32) {
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + slot * 8, &actor.to_le_bytes());
    memory.put_at_rva(
        HANDLE_POOL_GENERATIONS_RVA + slot * 4,
        &generation.to_le_bytes(),
    );
}

fn handle_bytes(slot: u32, generation: u32) -> Vec<u8> {
    let mut bytes = slot.to_le_bytes().to_vec();
    bytes.extend_from_slice(&generation.to_le_bytes());
    bytes
}

fn world() -> FakeMemory {
    let mut memory = FakeMemory::new();
    let empty_table = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * 8];
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA, &empty_table);
    memory.put_at_rva(BOOK_ACTOR_POINTER_RVA, &0_u64.to_le_bytes());
    pool_entry(&mut memory, PLAYER_SLOT, PLAYER, GENERATION);
    pool_entry(&mut memory, ENEMY_SLOT, ENEMY, GENERATION);
    memory
}

fn live_effect(memory: &mut FakeMemory, effect_slot: u32, alpha: f32) -> u32 {
    let instance = EFFECT_HEAP + u64::from(effect_slot) * 0x1000;
    let pool_slot = EFFECT_POOL_RVA + effect_slot * EFFECT_POOL_SLOT_STRIDE as u32;
    memory.put_at_rva(pool_slot, &instance.to_le_bytes());
    memory.put_at_rva(
        pool_slot + EFFECT_POOL_GENERATION_OFFSET as u32,
        &EFFECT_GENERATION.to_le_bytes(),
    );
    let style = [0.9_f32, 0.1, 0.1, alpha, 2.0, 2.0, 2.0, 2.0];
    for (index, value) in style.iter().enumerate() {
        memory.put(
            instance + EFFECT_INSTANCE_STYLE_OFFSET + index as u64 * 4,
            &value.to_le_bytes(),
        );
    }
    (EFFECT_GENERATION.cast_unsigned() << 13) | effect_slot
}

fn bullet(memory: &mut FakeMemory, index: u32, owner_slot: u32, drawn: bool) -> u64 {
    let actor = BULLET_HEAP + u64::from(index) * BULLET_STRIDE;
    let vtable = FAKE_MODULE_BASE + u64::from(NORMAL_BULLET_VTABLE_RVA);
    memory.put(actor, &vtable.to_le_bytes());
    memory.put(
        actor + NORMAL_BULLET_OWNER_HANDLE_OFFSET,
        &handle_bytes(owner_slot, GENERATION),
    );
    let slot = FIRST_BULLET_SLOT + index;
    memory.put(
        actor + ACTOR_SELF_HANDLE_OFFSET,
        &handle_bytes(slot, GENERATION),
    );
    pool_entry(memory, slot, actor, GENERATION);
    memory.put(actor + NORMAL_BULLET_STATE_OFFSET, &0_i32.to_le_bytes());
    memory.put(
        actor + NORMAL_BULLET_HIT_ID_OFFSET,
        &(1000 + index as i32).to_le_bytes(),
    );
    let record = RECORD_HEAP + u64::from(index) * 0x1000;
    memory.put(
        actor + NORMAL_BULLET_RECORD_POINTER_OFFSET,
        &record.to_le_bytes(),
    );
    for (lane, id) in EFFECT_IDS.iter().enumerate() {
        memory.put(
            record + NORMAL_BULLET_RECORD_EFFECT_IDS_OFFSET + lane as u64 * 4,
            &id.to_le_bytes(),
        );
    }
    for lane in 0..4_u64 {
        let handle = if drawn && lane < 2 {
            live_effect(memory, index * 4 + lane as u32, 1.0)
        } else {
            0xFFFF_E000
        };
        memory.put(
            actor + NORMAL_BULLET_EFFECT_HANDLES_OFFSET + lane * 4,
            &handle.to_le_bytes(),
        );
    }
    memory.put(
        actor + ACTOR_TRANSFORM_BLOCK_OFFSET,
        &[0_u8; ACTOR_TRANSFORM_BLOCK_BYTES],
    );
    for (row, value) in [4.0_f32, 5.0, 6.0].into_iter().enumerate() {
        memory.put(
            actor + ACTOR_TRANSFORM_BLOCK_OFFSET + row as u64 * 16 + 12,
            &value.to_le_bytes(),
        );
    }
    actor
}

fn sighting(slot: u16, hit_id: i32, x: f32) -> SightedBullet {
    let mut transform = [0.0_f32; 12];
    transform[3] = x;
    SightedBullet {
        actor: 0x1000,
        slot,
        hit_id,
        transform,
        look: BulletLook {
            effect_ids: EFFECT_IDS,
            styles: [[0.0; BULLET_STYLE_FLOATS]; 4],
        },
    }
}

#[test]
fn only_the_players_running_drawn_bullets_are_sighted_with_their_look() {
    let mut memory = world();
    let mine = bullet(&mut memory, 0, PLAYER_SLOT, true);
    bullet(&mut memory, 1, ENEMY_SLOT, true);
    bullet(&mut memory, 2, PLAYER_SLOT, false);
    let finished = bullet(&mut memory, 3, PLAYER_SLOT, true);
    memory.put(finished + NORMAL_BULLET_STATE_OFFSET, &1_i32.to_le_bytes());
    let sighted = bullets_of_player(&memory, PLAYER).unwrap();
    assert_eq!(sighted.len(), 1);
    assert_eq!(sighted[0].actor, mine);
    assert_eq!(sighted[0].slot, FIRST_BULLET_SLOT as u16);
    assert_eq!(sighted[0].hit_id, 1000);
    assert_eq!(sighted[0].transform[3], 4.0);
    assert_eq!(sighted[0].transform[11], 6.0);
    assert_eq!(sighted[0].look.effect_ids, EFFECT_IDS);
    assert!(style_is_set(&sighted[0].look.styles[0]));
    assert!(style_is_set(&sighted[0].look.styles[1]));
    assert!(!style_is_set(&sighted[0].look.styles[2]));
}

#[test]
fn no_more_than_the_limit_of_bullets_are_sighted() {
    let mut memory = world();
    for index in 0..8 {
        bullet(&mut memory, index, PLAYER_SLOT, true);
    }
    assert_eq!(
        bullets_of_player(&memory, PLAYER).unwrap().len(),
        MAX_SIGHTED_BULLETS
    );
}

#[test]
fn a_new_shot_sends_its_look_then_moves_with_a_speed_and_ends_when_gone() {
    let mut tracker = BulletTracker::default();
    let first = tracker.update(&[sighting(7, 1, 0.0)], 1000);
    assert_eq!(first.len(), 1);
    assert!(first[0].look.is_some());
    assert_eq!(first[0].velocity, [0.0; 3]);
    let second = tracker.update(&[sighting(7, 1, 10.0)], 1100);
    assert!(second[0].look.is_none());
    assert_eq!(second[0].velocity[0], 100.0);
    let last = tracker.update(&[], 1200);
    assert_eq!(last.len(), 1);
    assert!(last[0].ended);
    assert_eq!(last[0].instance, 7);
    assert!(tracker.is_empty());
}

#[test]
fn a_recycled_bullet_ends_the_old_shot_and_starts_a_new_one() {
    let mut tracker = BulletTracker::default();
    tracker.update(&[sighting(7, 1, 0.0)], 1000);
    let flights = tracker.update(&[sighting(7, 2, 50.0)], 1100);
    assert_eq!(flights.len(), 2);
    assert!(flights[0].ended);
    assert!(!flights[1].ended);
    assert!(flights[1].look.is_some());
    assert_eq!(flights[1].velocity, [0.0; 3]);
}

#[test]
fn the_look_is_repeated_so_a_lost_start_is_recovered() {
    let mut tracker = BulletTracker::default();
    let mut with_look = 0;
    for step in 0..=BULLET_LOOK_REPEAT_EVERY_MESSAGES {
        let flights = tracker.update(&[sighting(7, 1, step as f32)], 1000 + u64::from(step) * 100);
        with_look += usize::from(flights[0].look.is_some());
    }
    assert_eq!(with_look, 2);
}

#[test]
fn ending_everything_ends_every_tracked_shot_once() {
    let mut tracker = BulletTracker::default();
    tracker.update(&[sighting(1, 1, 0.0), sighting(2, 2, 0.0)], 1000);
    let ended = tracker.end_all();
    assert_eq!(ended.len(), 2);
    assert!(ended.iter().all(|flight| flight.ended));
    assert!(tracker.end_all().is_empty());
}

#[test]
fn a_partner_shot_that_goes_quiet_is_dropped_and_the_table_is_capped() {
    let mut table = BulletFlightTable::default();
    assert!(table.note(1, 0));
    assert!(table.note(2, 100));
    assert!(table.note(1, 500));
    assert_eq!(table.take_stale(BULLET_FLIGHT_STALE_MS + 100), vec![2]);
    assert_eq!(table.len(), 1);
    for instance in 10..10 + MAX_PARTNER_BULLET_FLIGHTS as u16 {
        table.note(instance, 700);
    }
    assert!(!table.note(99, 700));
    table.forget(10);
    assert!(table.note(99, 700));
    assert_eq!(table.take_all().len(), MAX_PARTNER_BULLET_FLIGHTS);
    assert!(table.is_empty());
}

#[test]
fn a_partner_effect_id_outside_the_wire_range_is_refused() {
    assert_eq!(partner_effect_id(0x2710), Some(0x2710));
    assert_eq!(partner_effect_id(1), Some(1));
    assert_eq!(
        partner_effect_id(MAX_BULLET_EFFECT_ID),
        Some(MAX_BULLET_EFFECT_ID as u32)
    );
    assert_eq!(partner_effect_id(0), None);
    assert_eq!(partner_effect_id(-5), None);
    assert_eq!(partner_effect_id(MAX_BULLET_EFFECT_ID + 1), None);
}

fn sample_at(x: f32, velocity_x: f32, received_ms: u64) -> FlightSample {
    let mut transform = [0.0_f32; 12];
    transform[3] = x;
    FlightSample {
        transform,
        velocity: [velocity_x, 0.0, 0.0],
        received_ms,
    }
}

#[test]
fn a_partner_shot_glides_between_its_last_two_flights() {
    let previous = sample_at(0.0, 10.0, 1000);
    let latest = sample_at(1.0, 10.0, 1040);
    let halfway = smoothed_transform(Some(&previous), &latest, 1020 + BULLET_RENDER_DELAY_MS);
    assert!((halfway[3] - 0.5).abs() < 1e-5);
    let at_start = smoothed_transform(Some(&previous), &latest, 1000 + BULLET_RENDER_DELAY_MS);
    assert!(at_start[3].abs() < 1e-5);
}

#[test]
fn a_partner_shot_continues_along_its_velocity_with_a_bounded_reach() {
    let previous = sample_at(0.0, 10.0, 1000);
    let latest = sample_at(1.0, 10.0, 1040);
    let after = smoothed_transform(Some(&previous), &latest, 1040 + BULLET_RENDER_DELAY_MS + 50);
    assert!((after[3] - 1.5).abs() < 1e-4);
    let far = smoothed_transform(Some(&previous), &latest, 100_000);
    let reach = 10.0 * MAX_EXTRAPOLATION_MS as f32 / 1000.0;
    assert!((far[3] - (1.0 + reach)).abs() < 1e-4);
}

#[test]
fn a_new_partner_shot_extrapolates_until_a_second_flight_arrives() {
    let only = sample_at(2.0, 20.0, 500);
    let shown = smoothed_transform(None, &only, 600);
    assert!((shown[3] - 4.0).abs() < 1e-4);
}

#[test]
fn two_flights_in_the_same_millisecond_do_not_divide_by_zero() {
    let previous = sample_at(0.0, 0.0, 1000);
    let latest = sample_at(1.0, 0.0, 1000);
    let shown = smoothed_transform(Some(&previous), &latest, 1000 + BULLET_RENDER_DELAY_MS - 1);
    assert!(shown[3].is_finite());
}
