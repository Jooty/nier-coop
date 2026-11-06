use crate::stay_together::{FollowRole, PARTNER_TIMEOUT_MS, distance_between};
use crate::zone_rules::{GUEST_VETO_TEXT, ZoneRadius};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub phase: u16,
    pub position: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct Companion {
    pub name: String,
    pub placement: Option<Placement>,
    pub age_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneChangeSource {
    PlayerChosen,
    Scripted,
    ExitDoorInteraction,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneChangeAsk<'a> {
    pub source: ZoneChangeSource,
    pub role: FollowRole,
    pub connected: bool,
    pub local: Option<Placement>,
    pub companions: &'a [Companion],
    pub radius: ZoneRadius,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZoneVerdict {
    Allow,
    OnlyHostMayChange,
    WaitingFor(String),
}

impl ZoneVerdict {
    pub fn prompt_text(&self) -> Option<String> {
        match self {
            Self::Allow => None,
            Self::OnlyHostMayChange => Some(GUEST_VETO_TEXT.to_owned()),
            Self::WaitingFor(name) => Some(format!("Waiting for {name} to come closer")),
        }
    }
}

pub fn judge_zone_change(ask: &ZoneChangeAsk<'_>) -> ZoneVerdict {
    if !ask.connected {
        return ZoneVerdict::Allow;
    }
    match ask.role {
        FollowRole::Guest => ZoneVerdict::OnlyHostMayChange,
        FollowRole::Unknown => ZoneVerdict::Allow,
        FollowRole::Host if ask.source == ZoneChangeSource::Scripted => ZoneVerdict::Allow,
        FollowRole::Host => first_companion_too_far(ask).map_or(ZoneVerdict::Allow, |name| {
            ZoneVerdict::WaitingFor(name.to_owned())
        }),
    }
}

fn first_companion_too_far<'a>(ask: &'a ZoneChangeAsk<'_>) -> Option<&'a str> {
    let local = ask.local?;
    ask.companions
        .iter()
        .find(|companion| !is_nearby(companion, local, ask.radius))
        .map(|companion| companion.name.as_str())
}

fn is_nearby(companion: &Companion, local: Placement, radius: ZoneRadius) -> bool {
    let Some(placement) = companion.placement else {
        return false;
    };
    companion.age_ms <= PARTNER_TIMEOUT_MS
        && placement.phase == local.phase
        && distance_between(local.position, placement.position)
            .is_some_and(|distance| distance <= radius.units())
}
