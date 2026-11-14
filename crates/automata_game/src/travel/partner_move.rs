use std::fmt;

use crate::travel::{
    BODY_STATE_THE_RESET_TELEPORT_IGNORES, DEATH_CHAIN_STATES, MOVE_TO_PARTNER_MIN_INTERVAL_MS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveAsker {
    Local,
    Partner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoToPartnerReadings {
    pub hero_present: bool,
    pub hero_state: Option<i32>,
    pub partner_body_present: bool,
    /// The scene is in play and no cutscene or event holds the local hero.
    pub local_free: bool,
    /// The other player's newest state is fresh, so the body stands where they are now.
    pub partner_state_fresh: bool,
    /// None until the other player's state has told its phase.
    pub same_phase: Option<bool>,
    pub ms_since_last_move: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoToPartnerRefusal {
    NoHero,
    NotFree,
    NoPartnerBody,
    PartnerStateStale,
    DifferentPhase,
    MovedTooRecently,
    HeroStateUnreadable,
    HeroInAStateTheTeleportIgnores,
    HeroDying,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BringPartnerRefusal {
    NotConnected,
    DifferentPhase,
}

impl fmt::Display for GoToPartnerRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHero => write!(formatter, "there is no hero yet"),
            Self::NotFree => write!(
                formatter,
                "a cutscene, an event or a scene change holds the hero"
            ),
            Self::NoPartnerBody => write!(formatter, "there is no partner body yet"),
            Self::PartnerStateStale => write!(
                formatter,
                "the other player's position is not fresh (their game is loading or silent)"
            ),
            Self::DifferentPhase => {
                write!(formatter, "the other player is in another scene")
            }
            Self::MovedTooRecently => write!(
                formatter,
                "the last move was less than {} s ago",
                MOVE_TO_PARTNER_MIN_INTERVAL_MS / 1000
            ),
            Self::HeroStateUnreadable => write!(formatter, "the hero's state cannot be read"),
            Self::HeroInAStateTheTeleportIgnores => write!(
                formatter,
                "the game's teleport does nothing while the hero is in state {BODY_STATE_THE_RESET_TELEPORT_IGNORES:#X}"
            ),
            Self::HeroDying => write!(formatter, "the hero is in the death chain"),
        }
    }
}

impl fmt::Display for BringPartnerRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => write!(formatter, "no other player is connected"),
            Self::DifferentPhase => write!(formatter, "the other player is in another scene"),
        }
    }
}

/// One rule for both askers: the local button and the other player's "come to me".
pub fn decide_go_to_partner(readings: &GoToPartnerReadings) -> Result<(), GoToPartnerRefusal> {
    if !readings.hero_present {
        return Err(GoToPartnerRefusal::NoHero);
    }
    if !readings.local_free {
        return Err(GoToPartnerRefusal::NotFree);
    }
    if readings.same_phase == Some(false) {
        return Err(GoToPartnerRefusal::DifferentPhase);
    }
    if !readings.partner_body_present {
        return Err(GoToPartnerRefusal::NoPartnerBody);
    }
    if !readings.partner_state_fresh || readings.same_phase.is_none() {
        return Err(GoToPartnerRefusal::PartnerStateStale);
    }
    if readings
        .ms_since_last_move
        .is_some_and(|since_ms| since_ms < MOVE_TO_PARTNER_MIN_INTERVAL_MS)
    {
        return Err(GoToPartnerRefusal::MovedTooRecently);
    }
    match readings.hero_state {
        None => Err(GoToPartnerRefusal::HeroStateUnreadable),
        Some(BODY_STATE_THE_RESET_TELEPORT_IGNORES) => {
            Err(GoToPartnerRefusal::HeroInAStateTheTeleportIgnores)
        }
        Some(state) if DEATH_CHAIN_STATES.contains(&state) => Err(GoToPartnerRefusal::HeroDying),
        Some(_) => Ok(()),
    }
}

pub fn decide_bring_partner(
    connected: bool,
    same_phase: Option<bool>,
) -> Result<(), BringPartnerRefusal> {
    if !connected {
        return Err(BringPartnerRefusal::NotConnected);
    }
    if same_phase == Some(false) {
        return Err(BringPartnerRefusal::DifferentPhase);
    }
    Ok(())
}

pub fn move_distance(from: [f32; 3], to: [f32; 3]) -> f32 {
    from.iter()
        .zip(to)
        .map(|(from, to)| (to - from) * (to - from))
        .sum::<f32>()
        .sqrt()
}

/// What the menu shows after a move to the partner was tried.
pub fn go_to_partner_line(asker: MoveAsker, outcome: &Result<(), String>) -> String {
    match (asker, outcome) {
        (MoveAsker::Local, Ok(())) => "moved to the partner".to_owned(),
        (MoveAsker::Local, Err(reason)) => format!("not moved: {reason}"),
        (MoveAsker::Partner, Ok(())) => "the other player brought you to them".to_owned(),
        (MoveAsker::Partner, Err(reason)) => {
            format!("the other player asked to bring you; not moved: {reason}")
        }
    }
}
