use automata_game::character::{Character, pick_distinct};

#[test]
fn launcher_ids_round_trip() {
    for character in Character::ALL {
        assert_eq!(
            Character::from_launcher_id(character.launcher_id()),
            Some(character)
        );
    }
    assert_eq!(Character::from_launcher_id("2b"), Some(Character::TwoB));
    assert_eq!(Character::from_launcher_id("9s"), Some(Character::NineS));
    assert_eq!(Character::from_launcher_id("a2"), Some(Character::A2));
    assert_eq!(Character::from_launcher_id("pascal"), None);
}

#[test]
fn object_ids_are_the_hero_rows_and_round_trip() {
    assert_eq!(Character::TwoB.object_id(), 0x10000);
    assert_eq!(Character::A2.object_id(), 0x10100);
    assert_eq!(Character::NineS.object_id(), 0x10200);
    for character in Character::ALL {
        assert_eq!(
            Character::from_object_id(character.object_id()),
            Some(character)
        );
    }
    assert_eq!(Character::from_object_id(0x10300), None);
}

#[test]
fn no_two_characters_share_an_id() {
    for first in Character::ALL {
        for second in Character::ALL {
            if first != second {
                assert_ne!(first.object_id(), second.object_id());
                assert_ne!(first.launcher_id(), second.launcher_id());
            }
        }
    }
}

#[test]
fn two_players_never_share_a_character() {
    for wanted in Character::ALL {
        for taken in Character::ALL {
            assert_ne!(pick_distinct(wanted, taken), taken);
        }
    }
}

#[test]
fn a_free_wish_is_kept() {
    assert_eq!(pick_distinct(Character::A2, Character::TwoB), Character::A2);
}
