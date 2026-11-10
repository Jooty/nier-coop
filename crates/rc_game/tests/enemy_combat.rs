use rc_game::enemies::{
    AGGRO_PUSH_RANGE, AGGRO_REFRESH_MS, ActorHandle, AggroPlanner, AggroPlayers, AggroTarget,
    AttackRecord, BossFight, BossFightChange, BossPick, EnemyHitLogLimiter,
    GUEST_HIT_REPEAT_WINDOW_MS, GuestHitGuard, HitVerdict, KillWatch, PLAYER_HIT_REPEAT_WINDOW_MS,
    PlayerHitGuard, choose_target, ghost_is_same_spawn, plan_ghosts, victim_refuses_hit,
};
use rc_game::offsets::{
    ACTOR_HIT_PROTECTION_SECONDS_OFFSET, ACTOR_KIND_MASK_OFFSET, ATTACK_RECORD_ATTACK_KIND_OFFSET,
    ATTACK_RECORD_ATTACKER_HANDLE_OFFSET, ATTACK_RECORD_FLAGS_OFFSET, ATTACK_RECORD_HIT_ID_OFFSET,
    ATTACK_RECORD_HP_DAMAGE_OFFSET, ATTACK_RECORD_OWNER_HANDLE_OFFSET, HANDLE_POOL_GENERATIONS_RVA,
    HANDLE_POOL_POINTERS_RVA, HIT_FLAG_IGNORES_ALL_PROTECTION,
    HIT_FLAG_IGNORES_HIT_PROTECTION_SECONDS, PROTECTED_KIND_BITS_ACTIVE_BYTE_RVA,
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

#[test]
fn one_swing_on_a_players_body_counts_once_however_often_it_repeats() {
    let mut guard = PlayerHitGuard::new();
    assert!(guard.counts(0, id(1, 1), 1));
    for frame in 1..=23 {
        assert!(!guard.counts(frame * 16, id(1, 1), 1));
    }
}

#[test]
fn a_repeating_swing_keeps_renewing_its_window() {
    let mut guard = PlayerHitGuard::new();
    assert!(guard.counts(0, id(1, 1), 1));
    let mut now = 0;
    while now < PLAYER_HIT_REPEAT_WINDOW_MS * 3 {
        now += PLAYER_HIT_REPEAT_WINDOW_MS - 1;
        assert!(!guard.counts(now, id(1, 1), 1));
    }
}

#[test]
fn a_new_swing_or_another_enemy_counts_again() {
    let mut guard = PlayerHitGuard::new();
    assert!(guard.counts(0, id(1, 1), 1));
    assert!(guard.counts(10, id(1, 1), 2));
    assert!(guard.counts(20, id(1, 2), 1));
}

#[test]
fn a_swing_counts_again_after_its_window_has_passed() {
    let mut guard = PlayerHitGuard::new();
    assert!(guard.counts(0, id(1, 1), 1));
    assert!(guard.counts(PLAYER_HIT_REPEAT_WINDOW_MS + 1, id(1, 1), 1));
}

#[test]
fn hits_on_a_body_without_a_swing_id_always_count() {
    let mut guard = PlayerHitGuard::new();
    assert!(guard.counts(0, id(1, 1), 0));
    assert!(guard.counts(1, id(1, 1), 0));
}

#[test]
fn a_guest_only_enemy_expires_after_the_grace_and_only_once() {
    let mut watch = rc_game::enemies::GuestOnlyWatch::new();
    assert!(watch.expired(1000, &[7, 8]).is_empty());
    assert!(
        watch
            .expired(
                1000 + rc_game::enemies::GUEST_ONLY_ENEMY_GRACE_MS - 1,
                &[7, 8]
            )
            .is_empty()
    );
    let expired = watch.expired(1000 + rc_game::enemies::GUEST_ONLY_ENEMY_GRACE_MS, &[7, 8]);
    assert_eq!(expired, vec![7, 8]);
}

#[test]
fn an_enemy_that_matches_again_starts_its_grace_over() {
    let mut watch = rc_game::enemies::GuestOnlyWatch::new();
    assert!(watch.expired(0, &[7]).is_empty());
    assert!(watch.expired(2000, &[]).is_empty());
    assert!(watch.expired(2500, &[7]).is_empty());
    assert!(
        watch
            .expired(2500 + rc_game::enemies::GUEST_ONLY_ENEMY_GRACE_MS - 1, &[7])
            .is_empty()
    );
}

fn ghost_ids(ids: &[EnemyId]) -> std::collections::HashSet<EnemyId> {
    ids.iter().copied().collect()
}

#[test]
fn a_hidden_copy_the_host_now_has_is_adopted() {
    let plan = plan_ghosts(
        &[(7, id(5, 0)), (8, id(5, 1))],
        &ghost_ids(&[id(5, 0)]),
        &std::collections::HashSet::new(),
    );
    assert_eq!(plan.adopt, vec![7]);
    assert!(plan.release.is_empty());
}

#[test]
fn a_hidden_copy_stays_while_the_host_never_had_its_group() {
    let plan = plan_ghosts(
        &[(7, id(6, 0))],
        &ghost_ids(&[]),
        &std::collections::HashSet::new(),
    );
    assert!(plan.adopt.is_empty());
    assert!(plan.release.is_empty());
}

#[test]
fn a_hidden_copy_stays_while_the_host_still_has_enemies_of_its_group() {
    let ended = std::collections::HashSet::from([6]);
    let plan = plan_ghosts(&[(9, id(6, 3))], &ghost_ids(&[id(6, 0)]), &ended);
    assert!(plan.adopt.is_empty());
    assert!(plan.release.is_empty());
}

#[test]
fn a_surplus_hidden_copy_goes_when_the_host_has_finished_the_group() {
    let ended = std::collections::HashSet::from([6]);
    let plan = plan_ghosts(&[(9, id(6, 3)), (10, id(7, 0))], &ghost_ids(&[]), &ended);
    assert_eq!(plan.release, vec![9]);
    assert!(plan.adopt.is_empty());
}

#[test]
fn a_hidden_copy_is_dropped_when_its_address_holds_a_different_spawn() {
    assert!(ghost_is_same_spawn(Some(41), 41));
    assert!(!ghost_is_same_spawn(Some(42), 41));
    assert!(!ghost_is_same_spawn(None, 41));
}

const VICTIM: u64 = 0x7_0000_0000;

#[test]
fn a_victim_in_its_protection_seconds_refuses_a_hit_that_does_not_ignore_them() {
    let mut memory = FakeMemory::new();
    memory.put(
        VICTIM + ACTOR_HIT_PROTECTION_SECONDS_OFFSET,
        &0.3_f32.to_le_bytes(),
    );
    assert!(victim_refuses_hit(&memory, VICTIM, 0, false));
    assert!(!victim_refuses_hit(
        &memory,
        VICTIM,
        HIT_FLAG_IGNORES_HIT_PROTECTION_SECONDS,
        false
    ));
    assert!(!victim_refuses_hit(
        &memory,
        VICTIM,
        HIT_FLAG_IGNORES_ALL_PROTECTION,
        false
    ));
}

#[test]
fn a_victim_with_no_protection_takes_the_hit() {
    let mut memory = FakeMemory::new();
    memory.put(
        VICTIM + ACTOR_HIT_PROTECTION_SECONDS_OFFSET,
        &0.0_f32.to_le_bytes(),
    );
    assert!(!victim_refuses_hit(&memory, VICTIM, 0, false));
}

#[test]
fn the_protected_kind_bit_refuses_only_while_the_game_switch_is_on() {
    let mut memory = FakeMemory::new();
    memory.put(VICTIM + ACTOR_KIND_MASK_OFFSET, &8_u32.to_le_bytes());
    memory.put_at_rva(PROTECTED_KIND_BITS_ACTIVE_BYTE_RVA, &[0]);
    assert!(!victim_refuses_hit(&memory, VICTIM, 0, false));
    memory.put_at_rva(PROTECTED_KIND_BITS_ACTIVE_BYTE_RVA, &[1]);
    assert!(victim_refuses_hit(&memory, VICTIM, 0, false));
}

#[test]
fn a_victim_that_is_the_boss_takes_a_hit_inside_its_protection_seconds() {
    let mut memory = FakeMemory::new();
    memory.put(
        VICTIM + ACTOR_HIT_PROTECTION_SECONDS_OFFSET,
        &0.3_f32.to_le_bytes(),
    );
    assert!(victim_refuses_hit(&memory, VICTIM, 0, false));
    assert!(!victim_refuses_hit(&memory, VICTIM, 0, true));
}

#[test]
fn the_protected_kind_bit_still_refuses_a_hit_on_the_boss() {
    let mut memory = FakeMemory::new();
    memory.put(VICTIM + ACTOR_KIND_MASK_OFFSET, &8_u32.to_le_bytes());
    memory.put_at_rva(PROTECTED_KIND_BITS_ACTIVE_BYTE_RVA, &[1]);
    assert!(victim_refuses_hit(&memory, VICTIM, 0, true));
}

#[test]
fn a_boss_fight_is_off_until_a_boss_message_is_raised() {
    let mut fight = BossFight::new();
    assert_eq!(fight.observe(3, &[(id(1, 0), 100, 28800)]), None);
    assert!(!fight.is_active());
    assert!(!fight.is_boss(id(1, 0)));
}

#[test]
fn a_raised_boss_message_picks_the_live_enemy_with_the_highest_max_hp() {
    let mut fight = BossFight::new();
    fight.raise(3);
    let enemies = [
        (id(1, 0), 50, 120),
        (id(2, 0), 9000, 28800),
        (id(3, 0), 0, 99999),
    ];
    let change = fight.observe(3, &enemies);
    let pick = BossPick {
        id: id(2, 0),
        max_hp: 28800,
    };
    assert_eq!(change, Some(BossFightChange::On(pick)));
    assert!(fight.is_boss(id(2, 0)));
    assert!(!fight.is_boss(id(1, 0)));
    assert_eq!(fight.observe(3, &enemies), None);
}

#[test]
fn the_boss_stays_picked_after_it_dies() {
    let mut fight = BossFight::new();
    fight.raise(3);
    fight.observe(3, &[(id(1, 0), 50, 120), (id(2, 0), 9000, 28800)]);
    assert_eq!(
        fight.observe(3, &[(id(1, 0), 50, 120), (id(2, 0), 0, 28800)]),
        None
    );
    assert!(fight.is_boss(id(2, 0)));
}

#[test]
fn a_phase_change_ends_the_boss_fight() {
    let mut fight = BossFight::new();
    fight.raise(3);
    fight.observe(3, &[(id(2, 0), 9000, 28800)]);
    assert_eq!(
        fight.observe(4, &[(id(2, 0), 9000, 28800)]),
        Some(BossFightChange::Off)
    );
    assert!(!fight.is_active());
    assert!(!fight.is_boss(id(2, 0)));
}

#[test]
fn a_paused_player_is_never_the_aggro_target() {
    let enemy = [0.0, 0.0, 0.0];
    let host_paused = AggroPlayers {
        host: None,
        guest: Some([5.0, 0.0, 0.0]),
    };
    assert_eq!(
        choose_target(enemy, host_paused, Some(AggroTarget::Host)),
        Some(AggroTarget::Guest)
    );
    let guest_paused = AggroPlayers {
        host: Some([9.0, 0.0, 0.0]),
        guest: None,
    };
    assert_eq!(
        choose_target(enemy, guest_paused, Some(AggroTarget::Guest)),
        Some(AggroTarget::Host)
    );
}

#[test]
fn with_both_players_paused_there_is_no_aggro_target() {
    let both_paused = AggroPlayers {
        host: None,
        guest: None,
    };
    assert_eq!(
        choose_target([0.0; 3], both_paused, Some(AggroTarget::Host)),
        None
    );
    let mut planner = AggroPlanner::new();
    assert!(
        planner
            .plan(0, &[(id(1, 0), [0.0; 3])], both_paused)
            .is_empty()
    );
}

#[test]
fn an_enemy_hit_is_logged_once_per_gap_for_each_attacker() {
    let first = id(6, 11);
    let second = id(6, 13);
    let mut limiter = EnemyHitLogLimiter::default();
    assert!(limiter.allow(1_000, first));
    assert!(!limiter.allow(1_100, first));
    assert!(limiter.allow(1_100, second));
    assert!(limiter.allow(1_500, first));
}

#[test]
fn the_planner_reports_who_each_pushed_enemy_targets() {
    let enemy = id(6, 11);
    let mut planner = AggroPlanner::new();
    planner.plan(
        0,
        &[(enemy, [0.0, 0.0, 0.0])],
        AggroPlayers {
            host: None,
            guest: Some([2.0, 0.0, 0.0]),
        },
    );
    assert_eq!(planner.targets().get(&enemy), Some(&AggroTarget::Guest));
}
