use automata_game::partner::{
    FreeReadings, GlobalGates, HeroGates, NotFree, ReadyLatch, not_free_reason,
};

fn free() -> FreeReadings {
    FreeReadings {
        in_play: true,
        hero: Some(HeroGates {
            player_controlled: true,
            computer_controlled: false,
            input_blank_switch: 0,
            input_blank_seconds: 0.0,
        }),
        globals: Some(GlobalGates {
            scene_input_off_word: 0,
            event_input_off_word: 0,
            controller_stopping_mode: 0,
        }),
    }
}

fn with_hero(change: impl FnOnce(&mut HeroGates)) -> FreeReadings {
    let mut readings = free();
    if let Some(hero) = readings.hero.as_mut() {
        change(hero);
    }
    readings
}

fn with_globals(change: impl FnOnce(&mut GlobalGates)) -> FreeReadings {
    let mut readings = free();
    if let Some(globals) = readings.globals.as_mut() {
        change(globals);
    }
    readings
}

#[test]
fn a_player_is_free_in_play_with_a_controlled_hero_and_no_gate_set() {
    assert_eq!(not_free_reason(&free()), None);
    assert!(free().is_free());
}

#[test]
fn a_scene_that_is_not_in_play_or_has_no_hero_is_not_free() {
    assert_eq!(
        not_free_reason(&FreeReadings {
            in_play: false,
            ..free()
        }),
        Some(NotFree::NotInPlay)
    );
    assert_eq!(
        not_free_reason(&FreeReadings {
            hero: None,
            ..free()
        }),
        Some(NotFree::NoHero)
    );
    assert_eq!(
        not_free_reason(&FreeReadings {
            globals: None,
            ..free()
        }),
        Some(NotFree::GatesUnreadable)
    );
}

#[test]
fn each_gate_the_game_blanks_input_by_holds_the_player() {
    assert_eq!(
        not_free_reason(&with_hero(|hero| hero.player_controlled = false)),
        Some(NotFree::HeroNotPlayerControlled)
    );
    assert_eq!(
        not_free_reason(&with_hero(|hero| hero.computer_controlled = true)),
        Some(NotFree::HeroComputerControlled)
    );
    assert_eq!(
        not_free_reason(&with_hero(|hero| hero.input_blank_switch = 1)),
        Some(NotFree::HeroInputBlankSwitch(1))
    );
    assert_eq!(
        not_free_reason(&with_hero(|hero| hero.input_blank_seconds = 0.5)),
        Some(NotFree::HeroInputBlankTimer)
    );
    assert_eq!(
        not_free_reason(&with_globals(
            |globals| globals.scene_input_off_word = 0x20_0000
        )),
        Some(NotFree::SceneInputOffBit)
    );
    assert_eq!(
        not_free_reason(&with_globals(
            |globals| globals.event_input_off_word = 0x2000_0000
        )),
        Some(NotFree::EventInputOffBit)
    );
    assert_eq!(
        not_free_reason(&with_globals(|globals| globals.controller_stopping_mode = 5)),
        Some(NotFree::ControllerStoppingMode(5))
    );
}

#[test]
fn other_bits_of_the_two_flag_words_do_not_hold_the_player() {
    let other_bits = with_globals(|globals| {
        globals.scene_input_off_word = !0x20_0000;
        globals.event_input_off_word = !0x2000_0000;
    });
    assert_eq!(not_free_reason(&other_bits), None);
}

#[test]
fn every_reason_reads_as_a_plain_line() {
    assert_eq!(NotFree::NotInPlay.to_string(), "the scene is not in play");
    assert!(NotFree::SceneInputOffBit.to_string().contains("0x200000"));
    assert!(NotFree::EventInputOffBit.to_string().contains("0x20000000"));
    assert!(
        NotFree::ControllerStoppingMode(5)
            .to_string()
            .ends_with("reads 5")
    );
}

#[test]
fn ready_starts_at_the_first_free_moment_and_lasts_until_play_ends() {
    let mut latch = ReadyLatch::default();
    assert!(!latch.observe(false, false));
    assert!(!latch.observe(true, false));
    assert!(latch.observe(true, true));
    assert!(latch.observe(true, false));
    assert!(!latch.observe(false, false));
    assert!(!latch.observe(true, false));
}
