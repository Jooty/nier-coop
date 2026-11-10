use crate::stay_together::{FollowRole, PARTNER_TIMEOUT_MS, distance_between};
use crate::zone_rules::{EXIT_DOOR_RADIUS_FRACTION, GUEST_VETO_TEXT, ZoneRadius};

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
    pub partner_game_open: bool,
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
            Self::WaitingFor(name) => Some(format!(
                "All players have to be there to change zones ({name} is missing)"
            )),
        }
    }
}

pub fn judge_zone_change(ask: &ZoneChangeAsk<'_>) -> ZoneVerdict {
    if !ask.connected || !ask.partner_game_open {
        return ZoneVerdict::Allow;
    }
    match ask.role {
        FollowRole::Guest => ZoneVerdict::OnlyHostMayChange,
        FollowRole::Unknown => ZoneVerdict::Allow,
        FollowRole::Host if ask.source == ZoneChangeSource::Scripted => ZoneVerdict::Allow,
        FollowRole::Host => missing_names(ask).map_or(ZoneVerdict::Allow, ZoneVerdict::WaitingFor),
    }
}

fn missing_names(ask: &ZoneChangeAsk<'_>) -> Option<String> {
    let local = ask.local?;
    let radius = if ask.source == ZoneChangeSource::ExitDoorInteraction {
        ZoneRadius::new(ask.radius.units() * EXIT_DOOR_RADIUS_FRACTION)
    } else {
        ask.radius
    };
    let names: Vec<&str> = ask
        .companions
        .iter()
        .filter(|companion| !is_nearby(companion, local, radius))
        .map(|companion| companion.name.as_str())
        .collect();
    (!names.is_empty()).then(|| names.join(", "))
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
