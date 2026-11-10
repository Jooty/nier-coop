use rc_game::enemies::{
    DamageTakenLedger, EXTRA_PLAYERS_WITH_ONE_GUEST, damage_dealt_bonus_percent,
    damage_taken_factor, guest_hit_hp_damage, scaled_hp_damage,
};

const ENEMY: (u64, i32) = (0x1000, 7);

#[test]
fn sixty_percent_more_health_means_a_hit_takes_five_eighths() {
    let factor = damage_taken_factor(60, EXTRA_PLAYERS_WITH_ONE_GUEST);
    assert!((factor - 0.625).abs() < 1e-6);
}

#[test]
fn zero_percent_changes_nothing() {
    assert!((damage_taken_factor(0, 1) - 1.0).abs() < 1e-6);
    assert_eq!(damage_dealt_bonus_percent(0, 1), 0);
    assert_eq!(scaled_hp_damage(100, 0), 100);
}

#[test]
fn health_setting_is_capped_at_three_hundred_percent() {
    assert_eq!(damage_taken_factor(5000, 1), damage_taken_factor(300, 1));
    assert!((damage_taken_factor(300, 1) - 0.25).abs() < 1e-6);
}

#[test]
fn damage_setting_is_capped_at_fifty_percent_per_extra_player() {
    assert_eq!(damage_dealt_bonus_percent(10, 1), 10);
    assert_eq!(damage_dealt_bonus_percent(900, 1), 50);
    assert_eq!(damage_dealt_bonus_percent(10, 3), 30);
}

#[test]
fn ten_percent_more_damage_rounds_to_the_nearest_point() {
    assert_eq!(scaled_hp_damage(100, 10), 110);
    assert_eq!(scaled_hp_damage(15, 10), 17);
    assert_eq!(scaled_hp_damage(4, 10), 4);
}

#[test]
fn a_hit_that_heals_or_misses_is_left_alone() {
    assert_eq!(scaled_hp_damage(0, 50), 0);
    assert_eq!(scaled_hp_damage(-30, 50), -30);
}

#[test]
fn a_new_enemy_gets_the_factor_once() {
    let mut ledger = DamageTakenLedger::new();
    assert_eq!(ledger.writes(&[(ENEMY, 1.0)], 0.625), vec![(ENEMY, 0.625)]);
    assert!(ledger.writes(&[(ENEMY, 0.625)], 0.625).is_empty());
    assert_eq!(ledger.scaled_count(), 1);
}

#[test]
fn changing_the_setting_rescales_from_the_original_not_the_scaled_value() {
    let mut ledger = DamageTakenLedger::new();
    ledger.writes(&[(ENEMY, 1.0)], 0.625);
    let writes = ledger.writes(&[(ENEMY, 0.625)], 0.5);
    assert_eq!(writes, vec![(ENEMY, 0.5)]);
}

#[test]
fn a_value_the_game_set_itself_becomes_the_new_original() {
    let mut ledger = DamageTakenLedger::new();
    ledger.writes(&[(ENEMY, 1.0)], 0.5);
    let writes = ledger.writes(&[(ENEMY, 2.0)], 0.5);
    assert_eq!(writes, vec![(ENEMY, 1.0)]);
}

#[test]
fn restoring_gives_back_every_original_once() {
    let mut ledger = DamageTakenLedger::new();
    let other = (0x2000, 9);
    ledger.writes(&[(ENEMY, 1.0), (other, 1.5)], 0.5);
    let mut restores = ledger.restores();
    restores.sort_by_key(|(key, _)| key.0);
    assert_eq!(restores, vec![(ENEMY, 1.0), (other, 1.5)]);
    assert!(ledger.restores().is_empty());
}

#[test]
fn an_enemy_that_left_is_forgotten() {
    let mut ledger = DamageTakenLedger::new();
    ledger.writes(&[(ENEMY, 1.0)], 0.5);
    ledger.writes(&[], 0.5);
    assert!(ledger.restores().is_empty());
    assert_eq!(ledger.scaled_count(), 0);
}

#[test]
fn a_factor_of_one_writes_nothing_for_an_untouched_enemy() {
    let mut ledger = DamageTakenLedger::new();
    assert!(ledger.writes(&[(ENEMY, 1.0)], 1.0).is_empty());
}

#[test]
fn going_back_to_zero_percent_restores_the_original() {
    let mut ledger = DamageTakenLedger::new();
    ledger.writes(&[(ENEMY, 1.0)], 0.625);
    assert_eq!(ledger.writes(&[(ENEMY, 0.625)], 1.0), vec![(ENEMY, 1.0)]);
}

#[test]
fn a_scaled_enemy_tells_the_scale_to_put_back_before_a_hit() {
    let mut ledger = DamageTakenLedger::new();
    ledger.writes(&[(ENEMY, 1.0)], 0.625);
    assert_eq!(ledger.wanted(ENEMY), Some(0.625));
    assert_eq!(ledger.wanted((0x2000, 9)), None);
}

#[test]
fn an_unscaled_enemy_needs_no_scale_put_back() {
    let mut ledger = DamageTakenLedger::new();
    ledger.writes(&[(ENEMY, 1.0)], 1.0);
    assert_eq!(ledger.wanted(ENEMY), None);
}

#[test]
fn a_guest_hit_on_an_enemy_that_resets_its_scale_is_scaled_at_the_source() {
    let (original, wanted) = (1.0, 0.625);
    assert_eq!(guest_hit_hp_damage(273, original, wanted), 171);
    assert_eq!(guest_hit_hp_damage(1, original, 0.1), 1);
    assert_eq!(guest_hit_hp_damage(0, original, wanted), 0);
    assert_eq!(guest_hit_hp_damage(100, 0.0, wanted), 100);
}

#[test]
fn an_enemy_seen_in_two_phases_is_written_once_never_at_the_compounded_scale() {
    let mut ledger = DamageTakenLedger::new();
    assert_eq!(ledger.writes(&[(ENEMY, 1.0)], 0.625), vec![(ENEMY, 0.625)]);
    assert!(ledger.writes(&[(ENEMY, 0.625)], 0.625).is_empty());
    assert_eq!(ledger.scaling(ENEMY), Some((1.0, 0.625)));
}

#[test]
fn a_forgotten_ledger_would_compound_the_scale() {
    let mut forgotten = DamageTakenLedger::new();
    assert_eq!(
        forgotten.writes(&[(ENEMY, 0.625)], 0.625),
        vec![(ENEMY, 0.390625)]
    );
}

#[test]
fn the_ledger_gives_the_original_and_wanted_scale_only_while_they_differ() {
    let mut ledger = DamageTakenLedger::new();
    assert_eq!(ledger.scaling(ENEMY), None);
    ledger.writes(&[(ENEMY, 1.0)], 0.625);
    assert_eq!(ledger.scaling(ENEMY), Some((1.0, 0.625)));
    ledger.writes(&[(ENEMY, 0.625)], 1.0);
    assert_eq!(ledger.scaling(ENEMY), None);
}
