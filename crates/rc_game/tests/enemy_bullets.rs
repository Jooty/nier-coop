use std::collections::HashMap;

use rc_game::bullet_flights::SightedBullet;
use rc_game::enemy_bullets::{
    ENEMY_BULLET_STALE_MS, EnemyBullet, EnemyBulletBoard, EnemyBulletKinds, EnemyBulletSender,
    EnemyBulletStep, MAX_SHOWN_ENEMY_BULLETS, MAX_SIGHTED_ENEMY_BULLETS, bullets_of_enemies,
};
use rc_game::offsets::{
    ACTOR_SELF_HANDLE_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET,
    EFFECT_POOL_GENERATION_OFFSET, EFFECT_POOL_RVA, EFFECT_POOL_SLOT_STRIDE,
    HANDLE_POOL_GENERATIONS_RVA, HANDLE_POOL_POINTERS_RVA, HANDLE_POOL_SLOT_COUNT,
    NORMAL_BULLET_EFFECT_HANDLES_OFFSET, NORMAL_BULLET_HIT_ID_OFFSET,
    NORMAL_BULLET_OWNER_HANDLE_OFFSET, NORMAL_BULLET_RECORD_EFFECT_IDS_OFFSET,
    NORMAL_BULLET_RECORD_POINTER_OFFSET, NORMAL_BULLET_STATE_OFFSET, NORMAL_BULLET_VTABLE_RVA,
};
use rc_proto::{BULLET_STYLE_FLOATS, BulletFlight, BulletLook, EnemyId};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const PLAYER: u64 = 0x2_0000_0000;
const SHADE: u64 = 0x2_3000_0000;
const OTHER_SHADE: u64 = 0x2_3100_0000;
const BULLET_HEAP: u64 = 0x2_4000_0000;
const BULLET_STRIDE: u64 = 0x2_0000;
const RECORD_HEAP: u64 = 0x2_6000_0000;
const EFFECT_HEAP: u64 = 0x2_8000_0000;
const PLAYER_SLOT: u32 = 5;
const SHADE_SLOT: u32 = 6;
const OTHER_SHADE_SLOT: u32 = 7;
const FIRST_BULLET_SLOT: u32 = 100;
const GENERATION: u32 = 9;
const EFFECT_GENERATION: i32 = 3;
const EFFECT_IDS: [i32; 4] = [0x1F7, 0x1FB, 0, 0];
const SHADE_ID: EnemyId = EnemyId {
    group: 3,
    ordinal: 1,
};
const OTHER_SHADE_ID: EnemyId = EnemyId {
    group: 3,
    ordinal: 4,
};

fn pool_entry(memory: &mut FakeMemory, slot: u32, actor: u64) {
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + slot * 8, &actor.to_le_bytes());
    memory.put_at_rva(
        HANDLE_POOL_GENERATIONS_RVA + slot * 4,
        &GENERATION.to_le_bytes(),
    );
}

fn handle_bytes(slot: u32) -> Vec<u8> {
    let mut bytes = slot.to_le_bytes().to_vec();
    bytes.extend_from_slice(&GENERATION.to_le_bytes());
    bytes
}

fn world() -> FakeMemory {
    let mut memory = FakeMemory::new();
    let empty_table = vec![0_u8; HANDLE_POOL_SLOT_COUNT as usize * 8];
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA, &empty_table);
    pool_entry(&mut memory, PLAYER_SLOT, PLAYER);
    pool_entry(&mut memory, SHADE_SLOT, SHADE);
    pool_entry(&mut memory, OTHER_SHADE_SLOT, OTHER_SHADE);
    memory
}

fn shared_shades() -> HashMap<u64, EnemyId> {
    HashMap::from([(SHADE, SHADE_ID), (OTHER_SHADE, OTHER_SHADE_ID)])
}

fn live_effect(memory: &mut FakeMemory, effect_slot: u32) -> u32 {
    let instance = EFFECT_HEAP + u64::from(effect_slot) * 0x1000;
    let pool_slot = EFFECT_POOL_RVA + effect_slot * EFFECT_POOL_SLOT_STRIDE as u32;
    memory.put_at_rva(pool_slot, &instance.to_le_bytes());
    memory.put_at_rva(
        pool_slot + EFFECT_POOL_GENERATION_OFFSET as u32,
        &EFFECT_GENERATION.to_le_bytes(),
    );
    (EFFECT_GENERATION.cast_unsigned() << 13) | effect_slot
}

fn bullet(memory: &mut FakeMemory, index: u32, owner_slot: u32) -> u64 {
    let actor = BULLET_HEAP + u64::from(index) * BULLET_STRIDE;
    let vtable = FAKE_MODULE_BASE + u64::from(NORMAL_BULLET_VTABLE_RVA);
    memory.put(actor, &vtable.to_le_bytes());
    memory.put(
        actor + NORMAL_BULLET_OWNER_HANDLE_OFFSET,
        &handle_bytes(owner_slot),
    );
    let slot = FIRST_BULLET_SLOT + index;
    memory.put(actor + ACTOR_SELF_HANDLE_OFFSET, &handle_bytes(slot));
    pool_entry(memory, slot, actor);
    memory.put(actor + NORMAL_BULLET_STATE_OFFSET, &0_i32.to_le_bytes());
    memory.put(
        actor + NORMAL_BULLET_HIT_ID_OFFSET,
        &(2000 + index as i32).to_le_bytes(),
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
        let handle = if lane == 0 {
            live_effect(memory, index * 4)
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
    actor
}

fn sighting(enemy: EnemyId, slot: u16, hit_id: i32, x: f32) -> EnemyBullet {
    let mut transform = [0.0_f32; 12];
    transform[3] = x;
    EnemyBullet {
        enemy,
        bullet: SightedBullet {
            actor: 0x1000,
            slot,
            hit_id,
            transform,
            look: BulletLook {
                effect_ids: EFFECT_IDS,
                styles: [[0.0; BULLET_STYLE_FLOATS]; 4],
            },
        },
    }
}

fn flight(instance: u16, with_look: bool, ended: bool) -> BulletFlight {
    BulletFlight {
        instance,
        transform: [0.0; 12],
        velocity: [0.0; 3],
        look: with_look.then_some(BulletLook {
            effect_ids: EFFECT_IDS,
            styles: [[0.0; BULLET_STYLE_FLOATS]; 4],
        }),
        ended,
    }
}

#[test]
fn only_bullets_fired_by_shared_enemies_are_sighted_with_their_enemy() {
    let mut memory = world();
    let shade_shot = bullet(&mut memory, 0, SHADE_SLOT);
    bullet(&mut memory, 1, PLAYER_SLOT);
    let other_shot = bullet(&mut memory, 2, OTHER_SHADE_SLOT);
    let finished = bullet(&mut memory, 3, SHADE_SLOT);
    memory.put(finished + NORMAL_BULLET_STATE_OFFSET, &1_i32.to_le_bytes());
    let sighted = bullets_of_enemies(&memory, &shared_shades()).unwrap();
    assert_eq!(sighted.len(), 2);
    assert_eq!(sighted[0].enemy, SHADE_ID);
    assert_eq!(sighted[0].bullet.actor, shade_shot);
    assert_eq!(sighted[0].bullet.look.effect_ids, EFFECT_IDS);
    assert_eq!(sighted[1].enemy, OTHER_SHADE_ID);
    assert_eq!(sighted[1].bullet.actor, other_shot);
}

#[test]
fn no_enemy_bullets_are_sighted_without_shared_enemies_or_past_the_limit() {
    let mut memory = world();
    for index in 0..20 {
        bullet(&mut memory, index, SHADE_SLOT);
    }
    assert!(
        bullets_of_enemies(&memory, &HashMap::new())
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        bullets_of_enemies(&memory, &shared_shades()).unwrap().len(),
        MAX_SIGHTED_ENEMY_BULLETS
    );
}

#[test]
fn a_sent_enemy_bullet_carries_its_enemy_from_first_flight_to_end() {
    let mut sender = EnemyBulletSender::default();
    let first = sender.update(&[sighting(SHADE_ID, 120, 1, 0.0)], 1000);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].enemy, SHADE_ID);
    assert!(first[0].flight.look.is_some());
    let moved = sender.update(&[sighting(SHADE_ID, 120, 1, 5.0)], 1050);
    assert_eq!(moved[0].enemy, SHADE_ID);
    assert!(!moved[0].flight.ended);
    let ended = sender.update(&[], 1100);
    assert_eq!(ended.len(), 1);
    assert!(ended[0].flight.ended);
    assert_eq!(ended[0].enemy, SHADE_ID);
    assert!(sender.is_empty());
}

#[test]
fn a_recycled_slot_ends_the_old_enemys_bullet_and_starts_the_new_enemys() {
    let mut sender = EnemyBulletSender::default();
    sender.update(&[sighting(SHADE_ID, 120, 1, 0.0)], 1000);
    let flights = sender.update(&[sighting(OTHER_SHADE_ID, 120, 2, 9.0)], 1050);
    assert_eq!(flights.len(), 2);
    assert!(flights[0].flight.ended);
    assert_eq!(flights[0].enemy, SHADE_ID);
    assert!(!flights[1].flight.ended);
    assert_eq!(flights[1].enemy, OTHER_SHADE_ID);
}

#[test]
fn ending_all_enemy_bullets_ends_each_with_its_enemy() {
    let mut sender = EnemyBulletSender::default();
    sender.update(
        &[
            sighting(SHADE_ID, 120, 1, 0.0),
            sighting(OTHER_SHADE_ID, 121, 2, 0.0),
        ],
        1000,
    );
    let ended = sender.end_all();
    assert_eq!(ended.len(), 2);
    assert!(ended.iter().all(|flight| flight.flight.ended));
    assert_eq!(ended[0].enemy, SHADE_ID);
    assert_eq!(ended[1].enemy, OTHER_SHADE_ID);
    assert!(sender.end_all().is_empty());
}

#[test]
fn the_guest_starts_an_enemy_bullet_only_with_its_look_then_moves_and_stops_it() {
    let mut board = EnemyBulletBoard::default();
    assert_eq!(
        board.note(&flight(7, false, false), 0),
        EnemyBulletStep::Ignore
    );
    assert_eq!(
        board.note(&flight(7, true, false), 10),
        EnemyBulletStep::Start
    );
    assert_eq!(
        board.note(&flight(7, false, false), 20),
        EnemyBulletStep::Move
    );
    assert_eq!(
        board.note(&flight(7, false, true), 30),
        EnemyBulletStep::Stop
    );
    assert_eq!(
        board.note(&flight(7, false, true), 40),
        EnemyBulletStep::Ignore
    );
    assert!(board.is_empty());
}

#[test]
fn the_guest_shows_no_more_than_the_limit_and_drops_quiet_bullets() {
    let mut board = EnemyBulletBoard::default();
    for instance in 0..MAX_SHOWN_ENEMY_BULLETS as u16 {
        assert_eq!(
            board.note(&flight(instance, true, false), 0),
            EnemyBulletStep::Start
        );
    }
    assert_eq!(
        board.note(&flight(999, true, false), 0),
        EnemyBulletStep::Full
    );
    board.note(&flight(0, false, false), ENEMY_BULLET_STALE_MS);
    let stale = board.take_stale(ENEMY_BULLET_STALE_MS);
    assert_eq!(stale.len(), MAX_SHOWN_ENEMY_BULLETS - 1);
    assert!(!stale.contains(&0));
    assert_eq!(board.take_all(), vec![0]);
}

#[test]
fn each_enemy_bullet_kind_is_logged_once() {
    let mut kinds = EnemyBulletKinds::default();
    assert!(kinds.first_sighting(EFFECT_IDS));
    assert!(!kinds.first_sighting(EFFECT_IDS));
    assert!(kinds.first_sighting([0x245, 0, 0, 0]));
}
