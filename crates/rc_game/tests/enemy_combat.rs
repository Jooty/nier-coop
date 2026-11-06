use rc_game::enemies::{
    AGGRO_PUSH_RANGE, AGGRO_REFRESH_MS, ActorHandle, AggroPlanner, AggroPlayers, AggroTarget,
    AttackRecord, GUEST_HIT_REPEAT_WINDOW_MS, GuestHitGuard, HitVerdict, KillWatch, choose_target,
};
use rc_game::offsets::{
    ATTACK_RECORD_ATTACK_KIND_OFFSET, ATTACK_RECORD_ATTACKER_HANDLE_OFFSET,
    ATTACK_RECORD_FLAGS_OFFSET, ATTACK_RECORD_HIT_ID_OFFSET, ATTACK_RECORD_HP_DAMAGE_OFFSET,
    ATTACK_RECORD_OWNER_HANDLE_OFFSET, HANDLE_POOL_GENERATIONS_RVA, HANDLE_POOL_POINTERS_RVA,
};
use rc_proto::{EnemyHit, EnemyId};

use crate::fake_memory::FakeMemory;

const RECORD: u64 = 0x4_0000_0000;
const ATTACK_RECORD_BYTES: usize = 0x108;
const BULLET: u64 = 0x5_0000_0000;
const PLAYER: u64 = 0x6_0000_0000;

fn id(group: u16, ordinal: u16) -> EnemyId {
    EnemyId { group, ordinal }
}

fn both_players(host: [f32; 3], guest: [f32; 3]) -> AggroPlayers {
    AggroPlayers {
        host: Some(host),
        guest: Some(guest),
    }
}

fn hit(hit_id: u32, hp_damage: u32) -> EnemyHit {
    EnemyHit {
        phase: 4,
        id: id(19, 2),
        hit_id,
        hp_damage,
        second_gauge: 0,
        third_gauge: 0,
        flags: 0,
        attack_kind: 2,
    }
}

fn put_handle(memory: &mut FakeMemory, at: u64, slot: u32, generation: u32) {
    memory.put(at, &slot.to_le_bytes());
    memory.put(at + 4, &generation.to_le_bytes());
}

fn put_pool_slot(memory: &mut FakeMemory, slot: u32, actor: u64, generation: u32) {
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + slot * 8, &actor.to_le_bytes());
    memory.put_at_rva(
        HANDLE_POOL_GENERATIONS_RVA + slot * 4,
        &generation.to_le_bytes(),
    );
}

#[test]
fn an_enemy_goes_for_the_nearer_living_player() {
    let enemy = [0.0, 0.0, 0.0];
    let players = both_players([8.0, 0.0, 0.0], [3.0, 0.0, 0.0]);
    assert_eq!(
        choose_target(enemy, players, None),
        Some(AggroTarget::Guest)
    );
    let guest_downed = AggroPlayers {
        guest: None,
        ..players
    };
    assert_eq!(
        choose_target(enemy, guest_downed, None),
        Some(AggroTarget::Host)
    );
}

#[test]
fn a_target_is_kept_until_the_other_player_is_clearly_nearer() {
    let enemy = [0.0, 0.0, 0.0];
    let nearly_level = both_players([4.0, 0.0, 0.0], [5.0, 0.0, 0.0]);
    assert_eq!(
        choose_target(enemy, nearly_level, Some(AggroTarget::Guest)),
        Some(AggroTarget::Guest)
    );
    let host_clearly_nearer = both_players([2.0, 0.0, 0.0], [6.0, 0.0, 0.0]);
    assert_eq!(
        choose_target(enemy, host_clearly_nearer, Some(AggroTarget::Guest)),
        Some(AggroTarget::Host)
    );
}

#[test]
fn players_out_of_reach_are_not_chosen() {
    let far = AGGRO_PUSH_RANGE + 1.0;
    let players = both_players([far, 0.0, 0.0], [0.0, 0.0, far]);
    assert_eq!(choose_target([0.0; 3], players, None), None);
}

#[test]
fn the_planner_pushes_the_guest_once_then_refreshes_and_hands_back_to_the_host() {
    let mut planner = AggroPlanner::new();
    let enemies = [(id(19, 0), [0.0, 0.0, 0.0]), (id(19, 1), [0.0, 0.0, 30.0])];
    let guest_nearer = both_players([8.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
    assert_eq!(
        planner.plan(0, &enemies, guest_nearer),
        vec![(id(19, 0), AggroTarget::Guest)]
    );
    assert!(planner.plan(100, &enemies, guest_nearer).is_empty());
    assert_eq!(
        planner.plan(AGGRO_REFRESH_MS, &enemies, guest_nearer),
        vec![(id(19, 0), AggroTarget::Guest)]
    );
    let host_nearer = both_players([1.0, 0.0, 0.0], [9.0, 0.0, 0.0]);
    assert_eq!(
        planner.plan(AGGRO_REFRESH_MS + 1, &enemies, host_nearer),
        vec![(id(19, 0), AggroTarget::Host)]
    );
    assert!(
        planner
            .plan(AGGRO_REFRESH_MS + 2, &enemies, host_nearer)
            .is_empty()
    );
}

#[test]
fn a_downed_guest_hands_its_enemies_back_to_the_host() {
    let mut planner = AggroPlanner::new();
    let enemies = [(id(20, 0), [0.0, 0.0, 0.0])];
    planner.plan(0, &enemies, both_players([9.0, 0.0, 0.0], [1.0, 0.0, 0.0]));
    let downed = AggroPlayers {
        host: Some([9.0, 0.0, 0.0]),
        guest: None,
    };
    assert_eq!(
        planner.plan(10, &enemies, downed),
        vec![(id(20, 0), AggroTarget::Host)]
    );
    assert_eq!(planner.pushed_to_guest(), 0);
}

#[test]
fn the_planner_never_pushes_an_enemy_only_the_host_is_near() {
    let mut planner = AggroPlanner::new();
    let enemies = [(id(19, 0), [0.0, 0.0, 0.0])];
    let host_only = AggroPlayers {
        host: Some([1.0, 0.0, 0.0]),
        guest: None,
    };
    assert!(planner.plan(0, &enemies, host_only).is_empty());
}

#[test]
fn a_death_is_reported_once_when_hp_reaches_zero() {
    let mut watch = KillWatch::new();
    assert!(watch.observe(4, &[(id(19, 0), 30)], &[]).is_empty());
    assert_eq!(watch.observe(4, &[(id(19, 0), 0)], &[]), vec![id(19, 0)]);
    assert!(watch.observe(4, &[(id(19, 0), 0)], &[]).is_empty());
    assert!(watch.observe(4, &[], &[id(19, 0)]).is_empty());
}

#[test]
fn a_death_seen_only_as_a_killed_departure_is_still_reported() {
    let mut watch = KillWatch::new();
    watch.observe(4, &[(id(19, 1), 30)], &[]);
    assert_eq!(watch.observe(4, &[], &[id(19, 1)]), vec![id(19, 1)]);
}

#[test]
fn an_enemy_first_seen_dead_is_not_a_new_death() {
    let mut watch = KillWatch::new();
    assert!(watch.observe(4, &[(id(19, 2), 0)], &[]).is_empty());
}

#[test]
fn one_swing_lands_on_one_enemy_once() {
    let mut guard = GuestHitGuard::new();
    assert_eq!(guard.judge(0, 4, &hit(77, 30)), HitVerdict::Apply);
    assert_eq!(guard.judge(10, 4, &hit(77, 30)), HitVerdict::Repeat);
    assert_eq!(guard.judge(20, 4, &hit(78, 30)), HitVerdict::Apply);
    assert_eq!(
        guard.judge(GUEST_HIT_REPEAT_WINDOW_MS + 10, 4, &hit(77, 30)),
        HitVerdict::Apply
    );
}

#[test]
fn hits_without_a_swing_id_are_never_repeats() {
    let mut guard = GuestHitGuard::new();
    assert_eq!(guard.judge(0, 4, &hit(0, 30)), HitVerdict::Apply);
    assert_eq!(guard.judge(1, 4, &hit(0, 30)), HitVerdict::Apply);
}

#[test]
fn hits_from_another_phase_or_without_damage_are_refused() {
    let mut guard = GuestHitGuard::new();
    assert_eq!(guard.judge(0, 5, &hit(1, 30)), HitVerdict::WrongPhase);
    assert_eq!(guard.judge(0, 4, &hit(2, 0)), HitVerdict::NoDamage);
}

#[test]
fn an_attack_record_names_its_attacker_and_bullet_owner() {
    let mut memory = FakeMemory::new();
    memory.put(RECORD, &[0; ATTACK_RECORD_BYTES]);
    put_handle(
        &mut memory,
        RECORD + ATTACK_RECORD_ATTACKER_HANDLE_OFFSET,
        9,
        3,
    );
    put_handle(
        &mut memory,
        RECORD + ATTACK_RECORD_OWNER_HANDLE_OFFSET,
        12,
        1,
    );
    memory.put(RECORD + ATTACK_RECORD_HIT_ID_OFFSET, &41_u32.to_le_bytes());
    memory.put(
        RECORD + ATTACK_RECORD_HP_DAMAGE_OFFSET,
        &35_i32.to_le_bytes(),
    );
    memory.put(RECORD + ATTACK_RECORD_FLAGS_OFFSET, &0x21_u32.to_le_bytes());
    memory.put(
        RECORD + ATTACK_RECORD_ATTACK_KIND_OFFSET,
        &6_i32.to_le_bytes(),
    );
    put_pool_slot(&mut memory, 9, BULLET, 3);
    put_pool_slot(&mut memory, 12, PLAYER, 1);
    let record = AttackRecord::read(&memory, RECORD).expect("record");
    assert_eq!(record.hit_id, 41);
    assert_eq!(record.hp_damage, 35);
    assert_eq!(record.flags, 0x21);
    assert_eq!(record.attack_kind, 6);
    assert_eq!(record.sources(&memory), [Some(BULLET), Some(PLAYER)]);
}

#[test]
fn a_handle_to_a_reused_slot_resolves_to_nothing() {
    let mut memory = FakeMemory::new();
    put_pool_slot(&mut memory, 5, PLAYER, 2);
    let stale = ActorHandle {
        index: 5,
        generation: 1,
    };
    assert_eq!(stale.resolve(&memory), None);
    let fresh = ActorHandle {
        index: 5,
        generation: 2,
    };
    assert_eq!(fresh.resolve(&memory), Some(PLAYER));
    let out_of_range = ActorHandle {
        index: u32::MAX,
        generation: 2,
    };
    assert_eq!(out_of_range.resolve(&memory), None);
}
