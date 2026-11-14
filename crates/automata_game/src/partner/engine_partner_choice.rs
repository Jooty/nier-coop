use std::fmt;

use crate::character::Character;
use crate::partner::{
    BUDDY_2B_FLAG_HASH, BUDDY_9S_FLAG_HASH, BUDDY_A2_FLAG_HASH,
    ENGINE_PARTNER_SPAWN_CODE_CHECKED_LEN, ENGINE_PARTNER_SPAWN_FUNCTION_START,
    LOAD_OF_A_FOUR_BYTE_VALUE_INTO_THE_SECOND_ARGUMENT_OPCODE,
    ORDER_THE_ENGINE_TESTS_THE_BUDDY_FLAGS_IN,
};

/// The four partner flags of the scene's flag set, as the engine's partner spawn reads them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuddyFlags {
    pub nine_s: bool,
    pub two_b: bool,
    pub a2: bool,
    pub auto: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuddyFlagWrite {
    Set(Character),
    Clear(Character),
}

/// What the hook knows at the moment the engine is about to make its own partner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnginePartnerReadings {
    /// The other player's character, while that player is linked and heard.
    pub wanted: Option<Character>,
    pub hero: Option<Character>,
    pub slot_is_empty: bool,
    pub flags: BuddyFlags,
    /// A spawn of the wanted character waits for nothing.
    pub wanted_data_is_loaded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineChoiceLeftAlone {
    NoPartnerWanted,
    SlotIsFull,
    EngineMakesNoPartner,
    WantedIsTheHero,
    WantedDataNotLoaded,
    EngineAlreadyMakesIt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnginePartnerChoice {
    LeftAlone(EngineChoiceLeftAlone),
    /// These flag writes make the engine's own spawn pick the wanted character.
    Steered(Vec<BuddyFlagWrite>),
}

impl BuddyFlags {
    pub fn any(self) -> bool {
        self.nine_s || self.two_b || self.a2 || self.auto
    }

    pub fn has(self, character: Character) -> bool {
        match character {
            Character::NineS => self.nine_s,
            Character::TwoB => self.two_b,
            Character::A2 => self.a2,
        }
    }

    /// The character the engine's spawn makes for this hero: the first set flag in its test
    /// order, and with only the automatic flag the other of 2B and 9S. One story stretch with an
    /// A2 hero picks 9S by another test; that case is not modelled.
    pub fn engine_makes(self, hero: Option<Character>) -> Option<Character> {
        ORDER_THE_ENGINE_TESTS_THE_BUDDY_FLAGS_IN
            .into_iter()
            .find(|character| self.has(*character))
            .or_else(|| {
                self.auto.then_some(if hero == Some(Character::TwoB) {
                    Character::NineS
                } else {
                    Character::TwoB
                })
            })
    }

    pub fn describe(self) -> String {
        let names: Vec<&str> = [
            (self.nine_s, "Buddy/9S"),
            (self.two_b, "Buddy/2B"),
            (self.a2, "Buddy/A2"),
            (self.auto, "Buddy/Auto"),
        ]
        .into_iter()
        .filter_map(|(set, name)| set.then_some(name))
        .collect();
        if names.is_empty() {
            "none".to_owned()
        } else {
            names.join(", ")
        }
    }
}

impl fmt::Display for EngineChoiceLeftAlone {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoPartnerWanted => "no other player is linked and heard",
            Self::SlotIsFull => "the hero's partner slot is full",
            Self::EngineMakesNoPartner => "no partner flag is set, the engine makes no partner",
            Self::WantedIsTheHero => "the wanted character is the hero's own",
            Self::WantedDataNotLoaded => "the wanted character's data is not loaded",
            Self::EngineAlreadyMakesIt => "the flags already make the wanted character",
        })
    }
}

pub fn buddy_flag_hash(character: Character) -> u32 {
    match character {
        Character::NineS => BUDDY_9S_FLAG_HASH,
        Character::TwoB => BUDDY_2B_FLAG_HASH,
        Character::A2 => BUDDY_A2_FLAG_HASH,
    }
}

/// Decides whether the partner flags are changed before the engine's own partner spawn runs. The
/// flags are touched only where the engine is about to make a partner and the wanted character
/// can be made without a wait; everything else is left as the game has it.
pub fn choose_engine_partner(readings: &EnginePartnerReadings) -> EnginePartnerChoice {
    let left_alone = EnginePartnerChoice::LeftAlone;
    let Some(wanted) = readings.wanted else {
        return left_alone(EngineChoiceLeftAlone::NoPartnerWanted);
    };
    if !readings.slot_is_empty {
        return left_alone(EngineChoiceLeftAlone::SlotIsFull);
    }
    if !readings.flags.any() {
        return left_alone(EngineChoiceLeftAlone::EngineMakesNoPartner);
    }
    if readings.hero == Some(wanted) {
        return left_alone(EngineChoiceLeftAlone::WantedIsTheHero);
    }
    if readings.flags.engine_makes(readings.hero) == Some(wanted) {
        return left_alone(EngineChoiceLeftAlone::EngineAlreadyMakesIt);
    }
    if !readings.wanted_data_is_loaded {
        return left_alone(EngineChoiceLeftAlone::WantedDataNotLoaded);
    }
    let mut writes: Vec<BuddyFlagWrite> = ORDER_THE_ENGINE_TESTS_THE_BUDDY_FLAGS_IN
        .into_iter()
        .take_while(|character| *character != wanted)
        .filter(|character| readings.flags.has(*character))
        .map(BuddyFlagWrite::Clear)
        .collect();
    if !readings.flags.has(wanted) {
        writes.push(BuddyFlagWrite::Set(wanted));
    }
    EnginePartnerChoice::Steered(writes)
}

/// True when the bytes are the start of the engine's own partner spawn of either exe: its first
/// instructions, and the first flag test with the hash of "Buddy/9S" before its end.
pub fn engine_partner_spawn_code_is_the_noted_one(code: &[u8]) -> bool {
    let mut first_flag_test = vec![LOAD_OF_A_FOUR_BYTE_VALUE_INTO_THE_SECOND_ARGUMENT_OPCODE];
    first_flag_test.extend_from_slice(&BUDDY_9S_FLAG_HASH.to_le_bytes());
    code.len() >= ENGINE_PARTNER_SPAWN_CODE_CHECKED_LEN
        && code.starts_with(&ENGINE_PARTNER_SPAWN_FUNCTION_START)
        && code[..ENGINE_PARTNER_SPAWN_CODE_CHECKED_LEN]
            .windows(first_flag_test.len())
            .any(|window| window == first_flag_test)
}
