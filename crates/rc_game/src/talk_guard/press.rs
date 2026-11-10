use crate::callback_gate::event_base;
use crate::stay_together::{PARTNER_TIMEOUT_MS, distance_between};
use crate::talk_guard::{IMPORTANT_TALK_BASES, IMPORTANT_TALK_IDS};
use crate::zone_rules::{Companion, Placement, ZoneRadius};

#[derive(Debug, Clone, PartialEq)]
pub struct PressAsk<'a> {
    pub connected: bool,
    pub partner_game_open: bool,
    pub local: Option<Placement>,
    pub partner: Option<&'a Companion>,
    pub radius: ZoneRadius,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PressVerdict {
    Allow,
    Wait {
        partner: String,
        distance_units: f32,
    },
}

impl PressVerdict {
    pub fn prompt_text(&self) -> Option<String> {
        match self {
            Self::Allow => None,
            Self::Wait { partner, .. } => Some(format!(
                "All players have to be there to start this ({partner} is missing)"
            )),
        }
    }
}

pub fn is_important_talk(name: &str) -> bool {
    IMPORTANT_TALK_BASES
        .binary_search(&event_base(name))
        .is_ok()
}

pub fn important_base_of_talk_id(talk_id: i32) -> Option<&'static str> {
    let first = IMPORTANT_TALK_IDS.partition_point(|(id, _)| *id < talk_id);
    IMPORTANT_TALK_IDS
        .get(first)
        .filter(|(id, _)| *id == talk_id)
        .map(|(_, base)| *base)
}

pub fn judge_press(ask: &PressAsk<'_>) -> PressVerdict {
    if !ask.connected || !ask.partner_game_open {
        return PressVerdict::Allow;
    }
    let (Some(local), Some(partner)) = (ask.local, ask.partner) else {
        return PressVerdict::Allow;
    };
    let Some(placement) = partner.placement else {
        return PressVerdict::Allow;
    };
    if partner.age_ms > PARTNER_TIMEOUT_MS || placement.phase != local.phase {
        return PressVerdict::Allow;
    }
    match distance_between(local.position, placement.position) {
        Some(distance) if distance > ask.radius.units() => PressVerdict::Wait {
            partner: partner.name.clone(),
            distance_units: distance,
        },
        _ => PressVerdict::Allow,
    }
}
