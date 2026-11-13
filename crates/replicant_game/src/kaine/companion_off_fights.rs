/// A fight the companion sits out while a guest is in the session: from the first of its start
/// functions to the first of its end functions (or leaving its pack), she is kept inactive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionOffFight {
    pub name: &'static str,
    pub start_functions: &'static [&'static str],
    pub end_functions: &'static [&'static str],
}

pub const COMPANION_OFF_FIGHTS: [CompanionOffFight; 1] = [CompanionOffFight {
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
}];

pub fn companion_off_fight_started_by(function: &str) -> Option<&'static CompanionOffFight> {
    COMPANION_OFF_FIGHTS
        .iter()
        .find(|fight| fight.start_functions.contains(&function))
}
