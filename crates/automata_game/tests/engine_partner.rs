use automata_game::address::{
    ENGINE_PARTNER_SPAWN_FUNCTION, SCENE_STATE_CLEAR_FLAG_FUNCTION, SCENE_STATE_FLAG_TEST_FUNCTION,
};
use automata_game::character::Character;
use automata_game::exe::ExeVariant;
use automata_game::partner::{
    BUDDY_2B_FLAG_HASH, BUDDY_9S_FLAG_HASH, BUDDY_A2_FLAG_HASH, BUDDY_AUTO_FLAG_HASH,
    BuddyFlagWrite, BuddyFlags, ENGINE_PARTNER_SPAWN_CODE_CHECKED_LEN, EngineChoiceLeftAlone,
    EnginePartnerChoice, EnginePartnerReadings, PARTNER_DATA_WAIT_LIMIT_MS, PartnerDataReading,
    PartnerDataWait, buddy_flag_hash, choose_engine_partner,
    engine_partner_spawn_code_is_the_noted_one,
};
use automata_game::start::buddy_flag;

const MAIN_EXE_SPAWN_START: [u8; 0x70] = [
    0x40, 0x55, 0x53, 0x56, 0x48, 0x8D, 0x6C, 0x24, 0xB0, 0x48, 0x81, 0xEC, 0x50, 0x01, 0x00, 0x00,
    0x48, 0x8B, 0x05, 0x21, 0x64, 0xA5, 0x00, 0x48, 0x33, 0xC4, 0x48, 0x89, 0x45, 0x10, 0x48, 0x8D,
    0xB1, 0x7C, 0x64, 0x01, 0x00, 0x48, 0x8B, 0xD9, 0x48, 0x8B, 0xD6, 0x48, 0x8D, 0x4C, 0x24, 0x70,
    0xE8, 0xAB, 0x68, 0x22, 0x00, 0x48, 0x8D, 0x4C, 0x24, 0x70, 0xE8, 0x21, 0x77, 0x22, 0x00, 0x48,
    0x85, 0xC0, 0x0F, 0x85, 0xC5, 0x06, 0x00, 0x00, 0x48, 0x89, 0xBC, 0x24, 0x78, 0x01, 0x00, 0x00,
    0x48, 0x8D, 0x0D, 0x69, 0x3E, 0xAA, 0x00, 0x4C, 0x89, 0xA4, 0x24, 0x80, 0x01, 0x00, 0x00, 0xBA,
    0x8B, 0x1A, 0xDD, 0x96, 0x4C, 0x89, 0xB4, 0x24, 0x48, 0x01, 0x00, 0x00, 0x4C, 0x89, 0xBC, 0x24,
];
const COMPAT_EXE_SPAWN_START: [u8; 0x70] = [
    0x40, 0x55, 0x53, 0x56, 0x48, 0x8D, 0x6C, 0x24, 0xD0, 0x48, 0x81, 0xEC, 0x30, 0x01, 0x00, 0x00,
    0x48, 0x8D, 0xB1, 0x7C, 0x64, 0x01, 0x00, 0x48, 0x8B, 0xD9, 0x48, 0x8B, 0xD6, 0x48, 0x8D, 0x4D,
    0x50, 0xE8, 0x0A, 0x4F, 0x22, 0x00, 0x48, 0x8D, 0x4D, 0x50, 0xE8, 0x81, 0x5D, 0x22, 0x00, 0x48,
    0x85, 0xC0, 0x0F, 0x85, 0xC9, 0x06, 0x00, 0x00, 0x48, 0x89, 0xBC, 0x24, 0x60, 0x01, 0x00, 0x00,
    0x48, 0x8D, 0x0D, 0x99, 0xE3, 0xA9, 0x00, 0x4C, 0x89, 0xA4, 0x24, 0x28, 0x01, 0x00, 0x00, 0xBA,
    0x8B, 0x1A, 0xDD, 0x96, 0x4C, 0x89, 0xB4, 0x24, 0x18, 0x01, 0x00, 0x00, 0x4C, 0x89, 0xBC, 0x24,
    0x10, 0x01, 0x00, 0x00, 0x0F, 0x29, 0xB4, 0x24, 0x00, 0x01, 0x00, 0x00, 0x0F, 0x29, 0xBC, 0x24,
];
const ONLY_AUTO: BuddyFlags = BuddyFlags {
    nine_s: false,
    two_b: false,
    a2: false,
    auto: true,
};
const ONLY_9S: BuddyFlags = BuddyFlags {
    nine_s: true,
    two_b: false,
    a2: false,
    auto: false,
};

fn crc32(text: &str) -> u32 {
    let mut crc = u32::MAX;
    for byte in text.bytes() {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn readings(hero: Character, wanted: Character, flags: BuddyFlags) -> EnginePartnerReadings {
    EnginePartnerReadings {
        wanted: Some(wanted),
        hero: Some(hero),
        slot_is_empty: true,
        flags,
        wanted_data_is_loaded: true,
    }
}

fn after(flags: BuddyFlags, writes: &[BuddyFlagWrite]) -> BuddyFlags {
    let mut flags = flags;
    for write in writes {
        let (character, value) = match write {
            BuddyFlagWrite::Set(character) => (character, true),
            BuddyFlagWrite::Clear(character) => (character, false),
        };
        match character {
            Character::NineS => flags.nine_s = value,
            Character::TwoB => flags.two_b = value,
            Character::A2 => flags.a2 = value,
        }
    }
    flags
}

fn not_loaded(wanted: Character) -> Option<PartnerDataReading> {
    Some(PartnerDataReading {
        wanted,
        loaded: false,
    })
}

fn loaded(wanted: Character) -> Option<PartnerDataReading> {
    Some(PartnerDataReading {
        wanted,
        loaded: true,
    })
}

#[test]
fn the_engine_partner_addresses_are_picked_per_variant() {
    assert_eq!(
        ENGINE_PARTNER_SPAWN_FUNCTION.rva(ExeVariant::Main),
        0x51E7C0
    );
    assert_eq!(
        ENGINE_PARTNER_SPAWN_FUNCTION.rva(ExeVariant::Compat),
        0x5181A0
    );
    assert_eq!(
        SCENE_STATE_FLAG_TEST_FUNCTION.rva(ExeVariant::Main),
        0x495A70
    );
    assert_eq!(
        SCENE_STATE_FLAG_TEST_FUNCTION.rva(ExeVariant::Compat),
        0x48FB00
    );
    assert_eq!(
        SCENE_STATE_CLEAR_FLAG_FUNCTION.rva(ExeVariant::Main),
        0x34F8E0
    );
    assert_eq!(
        SCENE_STATE_CLEAR_FLAG_FUNCTION.rva(ExeVariant::Compat),
        0x349CC0
    );
}

#[test]
fn the_flag_hashes_are_the_checksums_of_the_flag_names() {
    for character in Character::ALL {
        assert_eq!(buddy_flag_hash(character), crc32(buddy_flag(character)));
    }
    assert_eq!(BUDDY_9S_FLAG_HASH, crc32("Buddy/9S"));
    assert_eq!(BUDDY_2B_FLAG_HASH, crc32("Buddy/2B"));
    assert_eq!(BUDDY_A2_FLAG_HASH, crc32("Buddy/A2"));
    assert_eq!(BUDDY_AUTO_FLAG_HASH, crc32("Buddy/Auto"));
}

#[test]
fn the_spawn_code_of_both_exes_is_known_and_other_code_is_not() {
    assert_eq!(
        MAIN_EXE_SPAWN_START.len(),
        ENGINE_PARTNER_SPAWN_CODE_CHECKED_LEN
    );
    assert!(engine_partner_spawn_code_is_the_noted_one(
        &MAIN_EXE_SPAWN_START
    ));
    assert!(engine_partner_spawn_code_is_the_noted_one(
        &COMPAT_EXE_SPAWN_START
    ));
    let mut another_first_flag = MAIN_EXE_SPAWN_START;
    another_first_flag[0x60] ^= 1;
    assert!(!engine_partner_spawn_code_is_the_noted_one(
        &another_first_flag
    ));
    let mut another_start = COMPAT_EXE_SPAWN_START;
    another_start[0] = 0x48;
    assert!(!engine_partner_spawn_code_is_the_noted_one(&another_start));
    assert!(!engine_partner_spawn_code_is_the_noted_one(
        &MAIN_EXE_SPAWN_START[..0x40]
    ));
}

#[test]
fn with_the_automatic_flag_the_engine_makes_the_other_of_2b_and_9s() {
    assert_eq!(
        ONLY_AUTO.engine_makes(Some(Character::TwoB)),
        Some(Character::NineS)
    );
    assert_eq!(
        ONLY_AUTO.engine_makes(Some(Character::NineS)),
        Some(Character::TwoB)
    );
    assert_eq!(
        ONLY_AUTO.engine_makes(Some(Character::A2)),
        Some(Character::TwoB)
    );
    assert_eq!(
        BuddyFlags::default().engine_makes(Some(Character::TwoB)),
        None
    );
}

#[test]
fn every_pairing_ends_with_flags_that_make_the_engine_pick_the_wanted_partner() {
    let all_flag_sets = (0..16_u8).filter(|bits| *bits != 0).map(|bits| BuddyFlags {
        nine_s: bits & 1 != 0,
        two_b: bits & 2 != 0,
        a2: bits & 4 != 0,
        auto: bits & 8 != 0,
    });
    for flags in all_flag_sets {
        for hero in Character::ALL {
            for wanted in Character::ALL.into_iter().filter(|wanted| *wanted != hero) {
                let made = match choose_engine_partner(&readings(hero, wanted, flags)) {
                    EnginePartnerChoice::Steered(writes) => {
                        assert!(!writes.is_empty());
                        after(flags, &writes).engine_makes(Some(hero))
                    }
                    EnginePartnerChoice::LeftAlone(reason) => {
                        assert_eq!(reason, EngineChoiceLeftAlone::EngineAlreadyMakesIt);
                        flags.engine_makes(Some(hero))
                    }
                };
                assert_eq!(made, Some(wanted), "{flags:?} {hero:?} {wanted:?}");
            }
        }
    }
}

#[test]
fn a2_for_a_2b_hero_clears_the_flags_tested_first_and_keeps_the_automatic_one() {
    let flags = BuddyFlags {
        nine_s: true,
        two_b: false,
        a2: false,
        auto: true,
    };
    assert_eq!(
        choose_engine_partner(&readings(Character::TwoB, Character::A2, flags)),
        EnginePartnerChoice::Steered(vec![
            BuddyFlagWrite::Clear(Character::NineS),
            BuddyFlagWrite::Set(Character::A2),
        ])
    );
}

#[test]
fn the_flags_are_left_alone_whenever_the_engine_would_not_make_the_wanted_body_safely() {
    let base = readings(Character::TwoB, Character::A2, ONLY_9S);
    let left_alone = |readings: EnginePartnerReadings| match choose_engine_partner(&readings) {
        EnginePartnerChoice::LeftAlone(reason) => reason,
        EnginePartnerChoice::Steered(writes) => panic!("steered with {writes:?}"),
    };
    assert_eq!(
        left_alone(EnginePartnerReadings {
            wanted: None,
            ..base
        }),
        EngineChoiceLeftAlone::NoPartnerWanted
    );
    assert_eq!(
        left_alone(EnginePartnerReadings {
            slot_is_empty: false,
            ..base
        }),
        EngineChoiceLeftAlone::SlotIsFull
    );
    assert_eq!(
        left_alone(EnginePartnerReadings {
            flags: BuddyFlags::default(),
            ..base
        }),
        EngineChoiceLeftAlone::EngineMakesNoPartner
    );
    assert_eq!(
        left_alone(EnginePartnerReadings {
            hero: Some(Character::A2),
            ..base
        }),
        EngineChoiceLeftAlone::WantedIsTheHero
    );
    assert_eq!(
        left_alone(EnginePartnerReadings {
            wanted_data_is_loaded: false,
            ..base
        }),
        EngineChoiceLeftAlone::WantedDataNotLoaded
    );
    assert_eq!(
        left_alone(EnginePartnerReadings {
            wanted: Some(Character::NineS),
            wanted_data_is_loaded: false,
            ..base
        }),
        EngineChoiceLeftAlone::EngineAlreadyMakesIt
    );
}

#[test]
fn data_that_is_loaded_already_keeps_the_load_for_no_time_and_asks_for_nothing() {
    let mut wait = PartnerDataWait::default();
    let step = wait.step(0, loaded(Character::NineS));
    assert!(step.ready);
    assert_eq!(step.ask_for, None);
    assert_eq!(step.logs.len(), 1);
    assert!(wait.step(16, loaded(Character::NineS)).logs.is_empty());
}

#[test]
fn data_that_is_not_loaded_is_asked_for_once_and_the_load_waits_until_it_is_read() {
    let mut wait = PartnerDataWait::default();
    let first = wait.step(100, not_loaded(Character::A2));
    assert!(!first.ready);
    assert_eq!(first.ask_for, Some(Character::A2));
    let second = wait.step(116, not_loaded(Character::A2));
    assert!(!second.ready);
    assert_eq!(second.ask_for, None);
    assert!(second.logs.is_empty());
    let read = wait.step(900, loaded(Character::A2));
    assert!(read.ready);
    assert!(read.logs[0].contains("800 ms"), "{:?}", read.logs);
}

#[test]
fn data_that_is_not_read_within_the_limit_lets_the_load_go_on() {
    let mut wait = PartnerDataWait::default();
    assert!(!wait.step(0, not_loaded(Character::A2)).ready);
    assert!(
        !wait
            .step(PARTNER_DATA_WAIT_LIMIT_MS - 1, not_loaded(Character::A2))
            .ready
    );
    let late = wait.step(PARTNER_DATA_WAIT_LIMIT_MS, not_loaded(Character::A2));
    assert!(late.ready);
    assert_eq!(late.ask_for, None);
    assert!(late.logs[0].contains("stands in"), "{:?}", late.logs);
    assert!(
        wait.step(PARTNER_DATA_WAIT_LIMIT_MS + 16, not_loaded(Character::A2))
            .logs
            .is_empty()
    );
}

#[test]
fn a_refused_request_lets_the_load_go_on_at_once() {
    let mut wait = PartnerDataWait::default();
    assert_eq!(
        wait.step(0, not_loaded(Character::A2)).ask_for,
        Some(Character::A2)
    );
    wait.ask_was_refused();
    assert!(wait.step(16, not_loaded(Character::A2)).ready);
}

#[test]
fn a_partner_who_names_another_character_is_asked_for_within_the_same_limit() {
    let mut wait = PartnerDataWait::default();
    assert_eq!(
        wait.step(0, not_loaded(Character::A2)).ask_for,
        Some(Character::A2)
    );
    assert_eq!(
        wait.step(4_000, not_loaded(Character::NineS)).ask_for,
        Some(Character::NineS)
    );
    assert!(
        wait.step(PARTNER_DATA_WAIT_LIMIT_MS, not_loaded(Character::NineS))
            .ready
    );
}

#[test]
fn with_no_partner_character_to_read_ahead_the_load_never_waits() {
    let mut wait = PartnerDataWait::default();
    let step = wait.step(0, None);
    assert!(step.ready);
    assert_eq!(step.ask_for, None);
}
