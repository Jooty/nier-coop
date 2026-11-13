use crate::kaine::RowOrigin;
use crate::kaine::companion_off_fights_from_scripts::COMPANION_OFF_FIGHTS_FROM_SCRIPTS;

/// A fight the companion sits out while a guest is in the session: from the first of its start
/// functions (or its approach, by the story flags) to the first of its end functions (or leaving
/// its pack), she is kept inactive, except while she is lent back to the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionOffFight {
    pub name: &'static str,
    /// A row from the scripts opens only in its approach's pack with its approach's on flag on.
    pub origin: RowOrigin,
    pub start_functions: &'static [&'static str],
    pub end_functions: &'static [&'static str],
    pub approach: CompanionOffApproach,
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
        origin: RowOrigin::HandMade,
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
        origin: RowOrigin::HandMade,
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

/// The rows from the scripts that act: those no hand-made row covers (one sharing a start
/// function with it).
pub fn companion_off_fights_from_scripts_in_use() -> impl Iterator<Item = &'static CompanionOffFight>
{
    rows_no_hand_made_fight_covers(&COMPANION_OFF_FIGHTS_FROM_SCRIPTS)
}

pub fn rows_no_hand_made_fight_covers(
    rows: &'static [CompanionOffFight],
) -> impl Iterator<Item = &'static CompanionOffFight> {
    rows.iter().filter(|row| {
        !COMPANION_OFF_FIGHTS.iter().any(|hand| {
            hand.start_functions
                .iter()
                .any(|name| row.start_functions.contains(name))
        })
    })
}

/// The row from the scripts among `rows` that `function` opens: one of this pack whose on flag is
/// on.
pub fn fight_from_scripts_opened_by(
    rows: impl IntoIterator<Item = &'static CompanionOffFight>,
    function: &str,
    pack: &str,
    flag_on: impl Fn(u16) -> bool,
) -> Option<&'static CompanionOffFight> {
    rows.into_iter().find(|row| {
        row.approach.pack == pack
            && row.start_functions.contains(&function)
            && flag_on(row.approach.on_game_flag)
    })
}

/// The fight `function` opens in `pack`: a hand-made row by its name anywhere, else, when
/// `from_scripts`, a row from the scripts of this pack whose on flag is on.
pub fn companion_off_fight_opened_by(
    function: &str,
    pack: &str,
    from_scripts: bool,
    flag_on: impl Fn(u16) -> bool,
) -> Option<&'static CompanionOffFight> {
    companion_off_fight_started_by(function).or_else(|| {
        from_scripts
            .then(|| {
                fight_from_scripts_opened_by(
                    companion_off_fights_from_scripts_in_use(),
                    function,
                    pack,
                    flag_on,
                )
            })
            .flatten()
    })
}

pub fn companion_off_fights_approached(
    pack: &str,
    from_scripts: bool,
) -> impl Iterator<Item = &'static CompanionOffFight> {
    companion_off_fights_approached_in(pack).chain(
        companion_off_fights_from_scripts_in_use()
            .filter(move |row| from_scripts && row.approach.pack == pack),
    )
}
