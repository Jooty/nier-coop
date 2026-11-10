use automata_game::character::Character;
use automata_game::partner::{
    BuddyReading, HeroReading, PartnerAction, PartnerMachine, PartnerReadings,
};

const HERO: u32 = 0x1100;
const BODY: u32 = 0x2200;

fn hero() -> Option<HeroReading> {
    Some(HeroReading {
        handle: HERO,
        player_controlled: true,
        computer_controlled: false,
    })
}

fn buddy(character: Character) -> Option<BuddyReading> {
    Some(BuddyReading {
        handle: BODY,
        object_id: character.object_id(),
    })
}

fn seeking(buddy: Option<BuddyReading>) -> PartnerReadings {
    PartnerReadings {
        partner_in_play: true,
        both_free: true,
        wanted: Character::NineS,
        hero: hero(),
        buddy,
        other_slot_holder: None,
        held_object_id: None,
    }
}

fn holding() -> PartnerReadings {
    PartnerReadings {
        held_object_id: Some(Character::NineS.object_id()),
        ..seeking(buddy(Character::NineS))
    }
}

fn machine_holding_the_engines_partner() -> PartnerMachine {
    let mut machine = PartnerMachine::default();
    let step = machine.step(0, &seeking(buddy(Character::NineS)));
    assert_eq!(step.action, PartnerAction::Adopt);
    machine
}

#[test]
fn nothing_happens_while_the_other_player_is_not_in_play() {
    let mut machine = PartnerMachine::default();
    let readings = PartnerReadings {
        partner_in_play: false,
        ..seeking(buddy(Character::NineS))
    };
    let step = machine.step(0, &readings);
    assert_eq!(step.action, PartnerAction::Nothing);
    assert!(step.logs.is_empty());
}

#[test]
fn nothing_happens_until_the_hero_exists_and_is_player_controlled() {
    let mut machine = PartnerMachine::default();
    let no_hero = PartnerReadings {
        hero: None,
        ..seeking(None)
    };
    assert_eq!(machine.step(0, &no_hero).action, PartnerAction::Nothing);
    let computer_hero = PartnerReadings {
        hero: Some(HeroReading {
            handle: HERO,
            player_controlled: true,
            computer_controlled: true,
        }),
        ..seeking(None)
    };
    assert_eq!(
        machine.step(10_000, &computer_hero).action,
        PartnerAction::Nothing
    );
}

#[test]
fn the_engines_partner_of_the_wanted_character_is_taken_over() {
    let mut machine = PartnerMachine::default();
    let step = machine.step(0, &seeking(buddy(Character::NineS)));
    assert_eq!(step.action, PartnerAction::Adopt);
    assert_eq!(machine.held_body_handle(), Some(BODY));
    assert!(step.logs.iter().any(|line| line.contains("taking over")));
    assert_eq!(machine.step(16, &holding()).action, PartnerAction::Drive);
}

#[test]
fn an_empty_slot_is_waited_on_for_five_seconds_and_then_spawned_into() {
    let mut machine = PartnerMachine::default();
    let first = machine.step(1_000, &seeking(None));
    assert_eq!(first.action, PartnerAction::Nothing);
    assert_eq!(first.logs.len(), 1);
    let waiting = machine.step(5_999, &seeking(None));
    assert_eq!(waiting.action, PartnerAction::Nothing);
    assert!(waiting.logs.is_empty());
    let step = machine.step(6_000, &seeking(None));
    assert_eq!(step.action, PartnerAction::Spawn { object_id: 0x10200 });
}

#[test]
fn the_five_seconds_count_from_when_a_controlled_body_exists() {
    let mut machine = PartnerMachine::default();
    let no_hero = PartnerReadings {
        hero: None,
        ..seeking(None)
    };
    machine.step(0, &no_hero);
    machine.step(20_000, &no_hero);
    assert_eq!(
        machine.step(20_016, &seeking(None)).action,
        PartnerAction::Nothing
    );
    assert_eq!(
        machine.step(25_016, &seeking(None)).action,
        PartnerAction::Spawn { object_id: 0x10200 }
    );
}

#[test]
fn a_partner_of_the_wrong_character_is_removed_and_the_spawn_follows_at_once() {
    let mut machine = PartnerMachine::default();
    let step = machine.step(0, &seeking(buddy(Character::A2)));
    assert_eq!(step.action, PartnerAction::DestroySlotPartner);
    assert_eq!(
        machine.step(16, &seeking(buddy(Character::A2))).action,
        PartnerAction::Nothing
    );
    assert_eq!(
        machine.step(32, &seeking(None)).action,
        PartnerAction::Spawn { object_id: 0x10200 }
    );
}

#[test]
fn a_removal_that_did_not_work_is_tried_again_after_a_second() {
    let mut machine = PartnerMachine::default();
    machine.step(0, &seeking(buddy(Character::A2)));
    assert_eq!(
        machine.step(1_000, &seeking(buddy(Character::A2))).action,
        PartnerAction::DestroySlotPartner
    );
}

#[test]
fn a_failed_spawn_is_tried_again_after_five_seconds_not_every_tick() {
    let mut machine = PartnerMachine::default();
    machine.step(0, &seeking(None));
    assert!(matches!(
        machine.step(5_000, &seeking(None)).action,
        PartnerAction::Spawn { .. }
    ));
    assert_eq!(machine.spawn_failed("the entity table is full").len(), 1);
    assert_eq!(
        machine.step(5_016, &seeking(None)).action,
        PartnerAction::Nothing
    );
    assert!(matches!(
        machine.step(10_000, &seeking(None)).action,
        PartnerAction::Spawn { .. }
    ));
}

#[test]
fn a_spawned_body_is_held_and_driven() {
    let mut machine = PartnerMachine::default();
    machine.step(0, &seeking(None));
    machine.step(5_000, &seeking(None));
    assert_eq!(machine.spawned(BODY, HERO).len(), 1);
    assert_eq!(machine.held_body_handle(), Some(BODY));
    assert_eq!(machine.step(5_016, &holding()).action, PartnerAction::Drive);
}

#[test]
fn a_partner_that_left_or_went_silent_gets_the_body_given_back_with_its_saved_fields() {
    let mut machine = machine_holding_the_engines_partner();
    let gone = PartnerReadings {
        partner_in_play: false,
        ..holding()
    };
    let step = machine.step(100, &gone);
    assert_eq!(
        step.action,
        PartnerAction::Release {
            restore_fields: true,
            terminate: false,
            made_by_mod: false,
            mark_dismissed: false,
        }
    );
    assert!(step.logs[0].contains("is a guess"));
    assert_eq!(machine.held_body_handle(), None);
    assert_eq!(machine.step(116, &gone).action, PartnerAction::Nothing);
}

#[test]
fn a_dead_body_handle_after_a_load_starts_over_without_writing_to_it() {
    let mut machine = machine_holding_the_engines_partner();
    let loaded = PartnerReadings {
        held_object_id: None,
        buddy: None,
        hero: None,
        ..holding()
    };
    let step = machine.step(100, &loaded);
    assert_eq!(
        step.action,
        PartnerAction::Release {
            restore_fields: false,
            terminate: false,
            made_by_mod: false,
            mark_dismissed: false,
        }
    );
    assert_eq!(machine.held_body_handle(), None);
    let new_hero = PartnerReadings {
        hero: Some(HeroReading {
            handle: 0x3300,
            player_controlled: true,
            computer_controlled: false,
        }),
        ..seeking(None)
    };
    assert_eq!(machine.step(200, &new_hero).action, PartnerAction::Nothing);
    assert_eq!(
        machine.step(5_200, &new_hero).action,
        PartnerAction::Spawn { object_id: 0x10200 }
    );
}

#[test]
fn a_changed_hero_handle_starts_over() {
    let mut machine = machine_holding_the_engines_partner();
    let other_hero = PartnerReadings {
        hero: Some(HeroReading {
            handle: 0x3300,
            player_controlled: true,
            computer_controlled: false,
        }),
        ..holding()
    };
    assert_eq!(
        machine.step(100, &other_hero).action,
        PartnerAction::Release {
            restore_fields: true,
            terminate: false,
            made_by_mod: false,
            mark_dismissed: false,
        }
    );
}

#[test]
fn a_body_that_fell_out_of_the_slot_is_linked_again_once() {
    let out_of_slot = PartnerReadings {
        buddy: None,
        ..holding()
    };
    let mut machine = machine_holding_the_engines_partner();
    assert_eq!(
        machine.step(100, &out_of_slot).action,
        PartnerAction::Relink
    );
    assert_eq!(machine.step(116, &holding()).action, PartnerAction::Drive);
    assert_eq!(
        machine.step(132, &out_of_slot).action,
        PartnerAction::Relink
    );
}

#[test]
fn when_linking_again_did_not_help_an_adopted_body_is_given_back_and_a_spawned_one_removed() {
    let out_of_slot = PartnerReadings {
        buddy: None,
        ..holding()
    };
    let mut adopted = machine_holding_the_engines_partner();
    adopted.step(100, &out_of_slot);
    assert_eq!(
        adopted.step(116, &out_of_slot).action,
        PartnerAction::Release {
            restore_fields: true,
            terminate: false,
            made_by_mod: false,
            mark_dismissed: true,
        }
    );
    let mut spawned = PartnerMachine::default();
    spawned.spawned(BODY, HERO);
    spawned.step(100, &out_of_slot);
    assert_eq!(
        spawned.step(116, &out_of_slot).action,
        PartnerAction::Release {
            restore_fields: false,
            terminate: true,
            made_by_mod: true,
            mark_dismissed: false,
        }
    );
}

#[test]
fn a_slot_taken_by_another_body_is_not_linked_over() {
    let taken = PartnerReadings {
        buddy: Some(BuddyReading {
            handle: 0x9900,
            object_id: Character::NineS.object_id(),
        }),
        ..holding()
    };
    let mut machine = machine_holding_the_engines_partner();
    assert_eq!(
        machine.step(100, &taken).action,
        PartnerAction::Release {
            restore_fields: true,
            terminate: false,
            made_by_mod: false,
            mark_dismissed: true,
        }
    );
}

#[test]
fn a_held_body_of_another_character_than_the_partner_named_is_let_go_and_replaced() {
    let mut machine = machine_holding_the_engines_partner();
    let renamed = PartnerReadings {
        wanted: Character::A2,
        ..holding()
    };
    let step = machine.step(100, &renamed);
    assert_eq!(
        step.action,
        PartnerAction::Release {
            restore_fields: true,
            terminate: false,
            made_by_mod: false,
            mark_dismissed: false,
        }
    );
    let seeking_a2 = PartnerReadings {
        wanted: Character::A2,
        ..seeking(buddy(Character::NineS))
    };
    assert_eq!(
        machine.step(116, &seeking_a2).action,
        PartnerAction::DestroySlotPartner
    );
    let empty = PartnerReadings {
        wanted: Character::A2,
        ..seeking(None)
    };
    assert_eq!(
        machine.step(132, &empty).action,
        PartnerAction::Spawn { object_id: 0x10100 }
    );
}

#[test]
fn nothing_is_sought_while_a_player_is_not_free_and_the_wait_counts_from_both_free() {
    let mut machine = PartnerMachine::default();
    let loading = PartnerReadings {
        both_free: false,
        ..seeking(None)
    };
    for now_ms in [0, 3_000, 9_000] {
        let step = machine.step(now_ms, &loading);
        assert_eq!(step.action, PartnerAction::Nothing);
        assert!(step.logs.is_empty());
    }
    assert_eq!(
        machine.step(9_016, &seeking(None)).action,
        PartnerAction::Nothing
    );
    assert_eq!(
        machine.step(14_000, &seeking(None)).action,
        PartnerAction::Nothing
    );
    assert_eq!(
        machine.step(14_016, &seeking(None)).action,
        PartnerAction::Spawn { object_id: 0x10200 }
    );
}

#[test]
fn the_engines_partner_is_not_taken_over_while_a_player_is_not_free() {
    let mut machine = PartnerMachine::default();
    let loading = PartnerReadings {
        both_free: false,
        ..seeking(buddy(Character::NineS))
    };
    assert_eq!(machine.step(0, &loading).action, PartnerAction::Nothing);
    assert_eq!(machine.held_body_handle(), None);
}

#[test]
fn a_held_body_goes_back_to_the_game_while_a_player_is_not_free() {
    let mut machine = machine_holding_the_engines_partner();
    let in_a_cutscene = PartnerReadings {
        both_free: false,
        ..holding()
    };
    let step = machine.step(100, &in_a_cutscene);
    assert_eq!(
        step.action,
        PartnerAction::Release {
            restore_fields: true,
            terminate: false,
            made_by_mod: false,
            mark_dismissed: false,
        }
    );
    assert!(step.logs[0].contains("giving the body back to the game"));
    assert_eq!(machine.held_body_handle(), None);
    assert_eq!(
        machine.step(116, &in_a_cutscene).action,
        PartnerAction::Nothing
    );
    let free_again = PartnerReadings {
        held_object_id: None,
        ..holding()
    };
    assert_eq!(machine.step(132, &free_again).action, PartnerAction::Adopt);
}

#[test]
fn a_body_whose_handle_died_during_a_load_is_still_dropped() {
    let mut machine = machine_holding_the_engines_partner();
    let loading_without_the_body = PartnerReadings {
        both_free: false,
        held_object_id: None,
        ..holding()
    };
    assert!(matches!(
        machine.step(100, &loading_without_the_body).action,
        PartnerAction::Release {
            restore_fields: false,
            terminate: false,
            ..
        }
    ));
}

#[test]
fn a_body_the_mod_made_is_marked_so_when_it_is_given_back() {
    let mut machine = PartnerMachine::default();
    machine.spawned(BODY, HERO);
    let gone = PartnerReadings {
        partner_in_play: false,
        ..holding()
    };
    assert_eq!(
        machine.step(100, &gone).action,
        PartnerAction::Release {
            restore_fields: true,
            terminate: false,
            made_by_mod: true,
            mark_dismissed: false,
        }
    );
}

#[test]
fn a_slot_held_by_something_that_is_not_a_player_body_is_named_once_and_left_alone() {
    let mut machine = PartnerMachine::default();
    let other_holder = PartnerReadings {
        other_slot_holder: Some(0x7700),
        ..seeking(None)
    };
    let first = machine.step(0, &other_holder);
    assert_eq!(first.action, PartnerAction::Nothing);
    assert!(first.logs.iter().any(|line| line.contains("0x7700")));
    for now_ms in [5_000, 10_000, 60_000] {
        let step = machine.step(now_ms, &other_holder);
        assert_eq!(step.action, PartnerAction::Nothing);
        assert!(step.logs.is_empty());
    }
    assert_eq!(
        machine.step(60_016, &seeking(None)).action,
        PartnerAction::Spawn { object_id: 0x10200 }
    );
}
