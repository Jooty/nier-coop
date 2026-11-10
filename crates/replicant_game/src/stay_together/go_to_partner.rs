use crate::stay_together::PARTNER_TIMEOUT_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoToPartnerRefusal {
    NotConnected,
    PartnerSilent,
    OwnPhaseUnknown,
    DifferentPhase,
}

impl GoToPartnerRefusal {
    pub fn label(self) -> &'static str {
        match self {
            Self::NotConnected => "no other player is connected",
            Self::PartnerSilent => "the other player stopped sending",
            Self::OwnPhaseUnknown => "your own map is not readable (loading?)",
            Self::DifferentPhase => "the other player is in another map",
        }
    }
}

pub fn check_partner_reachable(
    connected: bool,
    partner_age_ms: Option<u64>,
    local_phase: Option<u16>,
    partner_phase: Option<u16>,
) -> Result<(), GoToPartnerRefusal> {
    let (true, Some(age), Some(partner_phase)) = (connected, partner_age_ms, partner_phase) else {
        return Err(GoToPartnerRefusal::NotConnected);
    };
    if age > PARTNER_TIMEOUT_MS {
        return Err(GoToPartnerRefusal::PartnerSilent);
    }
    match local_phase {
        None => Err(GoToPartnerRefusal::OwnPhaseUnknown),
        Some(phase) if phase != partner_phase => Err(GoToPartnerRefusal::DifferentPhase),
        Some(_) => Ok(()),
    }
}
