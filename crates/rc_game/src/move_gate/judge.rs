use crate::callback_gate::event_base;
use crate::move_gate::MOVING_EVENT_BASES;
use crate::stay_together::{PARTNER_TIMEOUT_MS, distance_between};
use crate::story_calls::is_event_callback_name;
use crate::zone_rules::{Companion, Placement, ZoneRadius};

#[derive(Debug, Clone, PartialEq)]
pub struct MoveAsk<'a> {
    pub connected: bool,
    pub partner_game_open: bool,
    pub local: Option<Placement>,
    pub partner: Option<&'a Companion>,
    pub radius: ZoneRadius,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveVerdict {
    Allow,
    Wait {
        partner: String,
        distance_units: f32,
    },
}

impl MoveVerdict {
    pub fn prompt_text(&self) -> Option<String> {
        match self {
            Self::Allow => None,
            Self::Wait { partner, .. } => Some(format!(
                "All players have to be there for this ({partner} is missing)"
            )),
        }
    }
}

pub fn moves_the_player(name: &str) -> bool {
    is_event_callback_name(name) && MOVING_EVENT_BASES.binary_search(&event_base(name)).is_ok()
}

pub fn judge_move(ask: &MoveAsk<'_>) -> MoveVerdict {
    if !ask.connected || !ask.partner_game_open {
        return MoveVerdict::Allow;
    }
    let (Some(local), Some(partner)) = (ask.local, ask.partner) else {
        return MoveVerdict::Allow;
    };
    let Some(placement) = partner.placement else {
        return MoveVerdict::Allow;
    };
    if partner.age_ms > PARTNER_TIMEOUT_MS || placement.phase != local.phase {
        return MoveVerdict::Allow;
    }
    match distance_between(local.position, placement.position) {
        Some(distance) if distance > ask.radius.units() => MoveVerdict::Wait {
            partner: partner.name.clone(),
            distance_units: distance,
        },
        _ => MoveVerdict::Allow,
    }
}
