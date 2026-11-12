use std::collections::BTreeMap;

use crate::enemies::{
    GUEST_LEAVE_TEST_FOLLOWS_THE_HOST_FOR_MS_AFTER_ITS_LAST_WORD, MOST_LEAVE_ANSWERS_KEPT,
};

/// Who says on this copy that the players have left a generator's area (the game's leave
/// test, which ends the generator's run and takes its enemies away).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeaveTestRule {
    #[default]
    TheGamesOwn,
    TheHostDecides,
    EitherPlayerKeepsTheRun,
}

/// What one leave test of a generator came to on this copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaveOutcome {
    PlayerIn,
    LeftByTheGamesOwnTest,
    HeldForTheHost,
    KeptByThePartner,
    BothLeft,
    LeftWithoutAPartnerBody,
}

/// The last outcome of each generator's leave test, so a line is logged only when it changes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LeaveWatch {
    last: BTreeMap<u32, LeaveOutcome>,
}

impl LeaveTestRule {
    pub fn as_byte(self) -> u8 {
        match self {
            Self::TheGamesOwn => 0,
            Self::TheHostDecides => 1,
            Self::EitherPlayerKeepsTheRun => 2,
        }
    }

    pub fn from_byte(byte: u8) -> Self {
        match byte {
            1 => Self::TheHostDecides,
            2 => Self::EitherPlayerKeepsTheRun,
            _ => Self::TheGamesOwn,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TheGamesOwn => "the game's own test on this copy's player",
            Self::TheHostDecides => {
                "the host decides: this copy's player walking away ends no run of a shared generator"
            }
            Self::EitherPlayerKeepsTheRun => {
                "a run ends only when both this copy's player and the partner body are out of the area"
            }
        }
    }

    /// The answer this copy gives the game (true: the players have left) from the game's own
    /// test on this copy's player and, on the host, the same test on the partner body.
    pub fn answer(self, own_player_left: bool, partner_left: Option<bool>) -> (bool, LeaveOutcome) {
        match self {
            Self::TheGamesOwn => (
                own_player_left,
                if own_player_left {
                    LeaveOutcome::LeftByTheGamesOwnTest
                } else {
                    LeaveOutcome::PlayerIn
                },
            ),
            Self::TheHostDecides => (
                false,
                if own_player_left {
                    LeaveOutcome::HeldForTheHost
                } else {
                    LeaveOutcome::PlayerIn
                },
            ),
            Self::EitherPlayerKeepsTheRun => match (own_player_left, partner_left) {
                (false, _) => (false, LeaveOutcome::PlayerIn),
                (true, Some(false)) => (false, LeaveOutcome::KeptByThePartner),
                (true, Some(true)) => (true, LeaveOutcome::BothLeft),
                (true, None) => (true, LeaveOutcome::LeftWithoutAPartnerBody),
            },
        }
    }
}

impl LeaveWatch {
    /// A line when a generator's outcome changes to one the mod made, or back from one.
    pub fn note(&mut self, generator: u32, outcome: LeaveOutcome) -> Option<String> {
        if self.last.len() >= MOST_LEAVE_ANSWERS_KEPT && !self.last.contains_key(&generator) {
            self.last.clear();
        }
        let before = self.last.insert(generator, outcome);
        if before == Some(outcome) {
            return None;
        }
        let was_the_mods = before.is_some_and(LeaveOutcome::is_the_mods);
        match outcome {
            LeaveOutcome::HeldForTheHost => Some(format!(
                "enemies: leave test: this copy's player is out of the area of generator {generator:#X}; its enemies stay until the host says they go"
            )),
            LeaveOutcome::KeptByThePartner => Some(format!(
                "enemies: leave test: the host's player is out of the area of generator {generator:#X} but the partner body is in it; its run and its enemies stay"
            )),
            LeaveOutcome::BothLeft => Some(format!(
                "enemies: leave test: both players are out of the area of generator {generator:#X}; the game ends its run and its enemies go"
            )),
            LeaveOutcome::LeftWithoutAPartnerBody => Some(format!(
                "enemies: leave test: the host's player is out of the area of generator {generator:#X} and no partner body is here; the game ends its run"
            )),
            LeaveOutcome::PlayerIn if was_the_mods => Some(format!(
                "enemies: leave test: this copy's player is back in the area of generator {generator:#X}"
            )),
            LeaveOutcome::PlayerIn | LeaveOutcome::LeftByTheGamesOwnTest => None,
        }
    }

    pub fn forget(&mut self) {
        self.last.clear();
    }
}

impl LeaveOutcome {
    pub fn is_the_mods(self) -> bool {
        matches!(self, Self::HeldForTheHost | Self::KeptByThePartner)
    }
}

/// The rule for this copy now. The guest holds its leave test only while its generators are held
/// and the host's word on enemies is fresh; without the link or the host it is the game's own.
pub fn leave_test_rule(
    is_host: bool,
    spawns_follow_the_host: bool,
    connected: bool,
    guest_generators_held: bool,
    ms_since_the_hosts_last_word: Option<u64>,
) -> LeaveTestRule {
    if !spawns_follow_the_host || !connected {
        return LeaveTestRule::TheGamesOwn;
    }
    if is_host {
        return LeaveTestRule::EitherPlayerKeepsTheRun;
    }
    let host_heard_lately = ms_since_the_hosts_last_word
        .is_some_and(|ms| ms <= GUEST_LEAVE_TEST_FOLLOWS_THE_HOST_FOR_MS_AFTER_ITS_LAST_WORD);
    if guest_generators_held && host_heard_lately {
        LeaveTestRule::TheHostDecides
    } else {
        LeaveTestRule::TheGamesOwn
    }
}
