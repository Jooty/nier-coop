use crate::party_frame::PARTY_FRAME_PEER_TIMEOUT_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartyStatus {
    Up,
    Downed,
    OtherArea,
    Disconnected,
}

pub fn party_status(
    peer_known: bool,
    peer_state_age_ms: u64,
    same_phase: bool,
    partner_downed: bool,
) -> PartyStatus {
    if !peer_known || peer_state_age_ms > PARTY_FRAME_PEER_TIMEOUT_MS {
        PartyStatus::Disconnected
    } else if !same_phase {
        PartyStatus::OtherArea
    } else if partner_downed {
        PartyStatus::Downed
    } else {
        PartyStatus::Up
    }
}

pub fn health_fraction(hp: u32, max_hp: u32) -> f32 {
    if max_hp == 0 {
        return 0.0;
    }
    (hp as f32 / max_hp as f32).clamp(0.0, 1.0)
}
