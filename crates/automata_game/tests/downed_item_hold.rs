use automata_game::downed::{
    AUTO_ITEM_USE_HEALTH_SHARE, AutoItemUseHold, AutoItemUseReading, DownedPhase, ItemUseAsker,
    auto_item_use_check_is_skipped, item_use_asker, item_use_is_held, own_item_use_is_held,
};
use automata_game::exe::ExeVariant;

const MAX_HEALTH: i32 = 800;
const STEP: f32 = 1.0;
const SMALL_RECOVERY: i32 = 0;
const FULL_RECOVERY: i32 = 3;
const RECOVERY_AND_CURE: i32 = 0x5A;
const ATTACK_UP_ITEM: i32 = 0x64;

fn downed_reading(game_cool_down: f32) -> AutoItemUseReading {
    AutoItemUseReading {
        chip_level: 0,
        health: 1,
        max_health: MAX_HEALTH,
        game_cool_down,
        step: STEP,
    }
}

#[test]
fn own_item_use_is_held_only_while_down_with_the_setting_on() {
    assert!(own_item_use_is_held(true, DownedPhase::Downed));
    assert!(!own_item_use_is_held(false, DownedPhase::Downed));
    assert!(!own_item_use_is_held(true, DownedPhase::Up));
    assert!(!own_item_use_is_held(true, DownedPhase::Defeated));
}

#[test]
fn every_recovery_item_is_held_while_the_hold_is_on() {
    for item in [SMALL_RECOVERY, 1, 2, FULL_RECOVERY, RECOVERY_AND_CURE] {
        assert!(item_use_is_held(true, item), "item {item:#X}");
    }
}

#[test]
fn a_standing_hero_uses_recovery_items_as_the_game_does() {
    assert!(!item_use_is_held(false, SMALL_RECOVERY));
    assert!(!item_use_is_held(false, FULL_RECOVERY));
}

#[test]
fn items_that_do_not_heal_stay_usable_while_down() {
    assert!(!item_use_is_held(true, ATTACK_UP_ITEM));
    assert!(!item_use_is_held(true, -1));
}

#[test]
fn the_asker_is_found_from_the_return_address_on_both_exes() {
    let cases = [
        (0x50_E81C, 0x50_867C, ItemUseAsker::AutoUseItemChip),
        (0x94_BC36, 0x94_3426, ItemUseAsker::QuickItemMenu),
        (0x9B_ABAE, 0x9B_22FE, ItemUseAsker::MenuItemList),
        (0x99_1C21, 0x98_9371, ItemUseAsker::MenuItemScreen),
        (0x9B_54D1, 0x9A_CC21, ItemUseAsker::MenuItemScreen),
        (0x33_8ECE, 0x33_343E, ItemUseAsker::ScriptCommand),
    ];
    for (main, compat, asker) in cases {
        assert_eq!(item_use_asker(ExeVariant::Main, main), asker);
        assert_eq!(item_use_asker(ExeVariant::Compat, compat), asker);
    }
}

#[test]
fn a_return_address_of_the_other_exe_is_unknown() {
    assert_eq!(
        item_use_asker(ExeVariant::Compat, 0x50_E81C),
        ItemUseAsker::Unknown
    );
    assert_eq!(item_use_asker(ExeVariant::Main, 0), ItemUseAsker::Unknown);
}

#[test]
fn the_held_chip_check_would_use_once_the_games_cool_down_runs_out() {
    let mut hold = AutoItemUseHold::new();
    assert!(!hold.held_check_would_use(downed_reading(2.5), AUTO_ITEM_USE_HEALTH_SHARE));
    assert!(!hold.held_check_would_use(downed_reading(2.5), AUTO_ITEM_USE_HEALTH_SHARE));
    assert!(hold.held_check_would_use(downed_reading(2.5), AUTO_ITEM_USE_HEALTH_SHARE));
}

#[test]
fn after_a_held_use_the_next_one_waits_a_whole_cool_down() {
    let mut hold = AutoItemUseHold::new();
    assert!(hold.held_check_would_use(downed_reading(0.0), AUTO_ITEM_USE_HEALTH_SHARE));
    let mut calls_until_the_next = 0;
    while !hold.held_check_would_use(downed_reading(0.0), AUTO_ITEM_USE_HEALTH_SHARE) {
        calls_until_the_next += 1;
        assert!(
            calls_until_the_next < 100,
            "the held check never came due again"
        );
    }
    assert_eq!(calls_until_the_next, 47);
}

#[test]
fn without_the_chip_no_held_use_is_reported() {
    let mut hold = AutoItemUseHold::new();
    let reading = AutoItemUseReading {
        chip_level: -1,
        ..downed_reading(0.0)
    };
    assert!(!hold.held_check_would_use(reading, AUTO_ITEM_USE_HEALTH_SHARE));
}

#[test]
fn health_at_or_over_the_share_reports_no_held_use() {
    let mut hold = AutoItemUseHold::new();
    let reading = AutoItemUseReading {
        health: MAX_HEALTH,
        ..downed_reading(0.0)
    };
    assert!(!hold.held_check_would_use(reading, AUTO_ITEM_USE_HEALTH_SHARE));
}

#[test]
fn the_chip_check_is_skipped_on_the_held_partner_body_at_all_times() {
    for downed_revive_on in [true, false] {
        let hero_held = own_item_use_is_held(downed_revive_on, DownedPhase::Up);
        assert!(auto_item_use_check_is_skipped(true, hero_held));
    }
}

#[test]
fn the_chip_check_runs_on_the_own_standing_hero() {
    let hero_held = own_item_use_is_held(true, DownedPhase::Up);
    assert!(!auto_item_use_check_is_skipped(false, hero_held));
}

#[test]
fn the_chip_check_is_skipped_on_the_own_downed_hero_with_the_setting_on() {
    let hero_held = own_item_use_is_held(true, DownedPhase::Downed);
    assert!(auto_item_use_check_is_skipped(false, hero_held));
    let hero_held_without_the_setting = own_item_use_is_held(false, DownedPhase::Downed);
    assert!(!auto_item_use_check_is_skipped(
        false,
        hero_held_without_the_setting
    ));
}
