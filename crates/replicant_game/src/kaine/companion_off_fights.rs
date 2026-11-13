/// A fight the companion sits out while a guest is in the session: from the first of its start
/// functions (or its approach, by the story flags) to the first of its end functions (or leaving
/// its pack), she is kept inactive, except while she is lent back to the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionOffFight {
    pub name: &'static str,
    pub start_functions: &'static [&'static str],
    pub end_functions: &'static [&'static str],
    pub approach: CompanionOffApproach,
    /// The game flag the fight's closing callback sets: once it is on, the fight is over even if
    /// no end function ran here (a story jump past the fight in the same pack).
    pub over_game_flag: u16,
    /// She is given back for each of these (they address unique actor -2) and a moment after.
    pub functions_that_address_her: &'static [&'static str],
}

/// The stretch before a fight's first function: in `pack`, with one game flag on and the next
/// off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionOffApproach {
    pub pack: &'static str,
    pub on_game_flag: u16,
    pub off_game_flag: u16,
}

pub const COMPANION_OFF_FIGHTS: [CompanionOffFight; 2] = [
    CompanionOffFight {
        name: "boss fight 6",
        start_functions: &[
            "EID_1050_a0020_Start",
            "EID_1050_a0020_PLAY_EVENT",
            "EID_1050_a0020_End",
            "EID_1050_a0025_Start",
            "EID_1050_a0020_Continue",
            "EID_1050_a0025g_Continue",
            "EID_1050_a0030_Continue",
            "EID_1050_a0035g_01_Continue",
            "EID_1050_a0035g_02_Continue",
            "EID_1050_a0045g_Continue",
            "EID_1050_a0050_Continue",
            "EID_1050_a0055g_01_Continue",
            "EID_1050_a0055g_02_Continue",
            "EID_1050_a0065g_Continue",
        ],
        end_functions: &["EID_1050_a0070_Start"],
        approach: CompanionOffApproach {
            pack: "A_CLIFF_VILLAGE_01",
            on_game_flag: 374,
            off_game_flag: 375,
        },
        over_game_flag: 386,
        functions_that_address_her: &[
            "EID_1050_a0020_End",
            "EID_1050_a0025_Start",
            "EID_1050_a0030_End",
            "EID_1050_a0035_a0040",
            "EID_1050_a0040_End",
            "EID_1050_a0045_Start",
            "EID_1050_a0050_01_End",
            "EID_1050_a0055_Start",
            "EID_1050_a0020_Continue",
            "EID_1050_a0025g_Continue",
            "EID_1050_a0030_Continue",
            "EID_1050_a0035g_01_Continue",
            "EID_1050_a0035g_02_Continue",
            "EID_1050_a0045g_Continue",
            "EID_1050_a0050_Continue",
            "EID_1050_a0055g_01_Continue",
            "EID_1050_a0055g_02_Continue",
            "EID_1050_a0065g_Continue",
        ],
    },
    CompanionOffFight {
        name: "boss fight 4",
        start_functions: &[
            "EID_1000_a0050_01_Start",
            "EID_1000_a0050_Continue",
            "EID_1000_a0055g_Continue",
            "EID_1000_a0055_01_Start",
            "EID_1000_a0065g_01_Start",
        ],
        end_functions: &["EID_1000_a0065g_01_End", "EID_1000_a0070_01_Start"],
        approach: CompanionOffApproach {
            pack: "A_CLIFF_VILLAGE_01",
            on_game_flag: 246,
            off_game_flag: 247,
        },
        over_game_flag: 248,
        functions_that_address_her: &[
            "EID_1000_a0050_Continue",
            "EID_1000_a0055g_Continue",
            "EID_1000_a0055_01_Start",
            "PUT_KAINE",
        ],
    },
];

pub fn companion_off_fight_started_by(function: &str) -> Option<&'static CompanionOffFight> {
    COMPANION_OFF_FIGHTS
        .iter()
        .find(|fight| fight.start_functions.contains(&function))
}

pub fn companion_off_fights_approached_in(
    pack: &str,
) -> impl Iterator<Item = &'static CompanionOffFight> {
    COMPANION_OFF_FIGHTS
        .iter()
        .filter(move |fight| fight.approach.pack == pack)
}
