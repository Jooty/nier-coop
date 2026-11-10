use std::fmt;

use crate::travel::BODY_STATE_THE_RESET_TELEPORT_IGNORES;

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoToPartnerRefusal {
    NoHero,
    NoPartnerBody,
    HeroStateUnreadable,
    HeroInAStateTheTeleportIgnores,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BringPartnerRefusal {
    NotConnected,
}

impl fmt::Display for GoToPartnerRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHero => write!(formatter, "there is no hero yet"),
            Self::NoPartnerBody => write!(formatter, "there is no partner body yet"),
            Self::HeroStateUnreadable => write!(formatter, "the hero's state cannot be read"),
            Self::HeroInAStateTheTeleportIgnores => write!(
                formatter,
                "the game's teleport does nothing while the hero is in state {BODY_STATE_THE_RESET_TELEPORT_IGNORES:#X}"
            ),
        }
    }
}

impl fmt::Display for BringPartnerRefusal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotConnected => write!(formatter, "no other player is connected"),
        }
    }
}

pub fn decide_go_to_partner(readings: &GoToPartnerReadings) -> Result<(), GoToPartnerRefusal> {
    if !readings.hero_present {
        return Err(GoToPartnerRefusal::NoHero);
    }
    if !readings.partner_body_present {
        return Err(GoToPartnerRefusal::NoPartnerBody);
    }
    match readings.hero_state {
        None => Err(GoToPartnerRefusal::HeroStateUnreadable),
        Some(BODY_STATE_THE_RESET_TELEPORT_IGNORES) => {
            Err(GoToPartnerRefusal::HeroInAStateTheTeleportIgnores)
        }
        Some(_) => Ok(()),
    }
}

pub fn decide_bring_partner(connected: bool) -> Result<(), BringPartnerRefusal> {
    if connected {
        Ok(())
    } else {
        Err(BringPartnerRefusal::NotConnected)
    }
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
