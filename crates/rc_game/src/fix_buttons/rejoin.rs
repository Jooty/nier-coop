use crate::stay_together::PhaseTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejoinRefusal {
    HostAreaNotKnownYet,
    HostMovedSinceLastAnnounce,
}

impl RejoinRefusal {
    pub fn label(self) -> &'static str {
        match self {
            Self::HostAreaNotKnownYet => "the host has not announced its area yet",
            Self::HostMovedSinceLastAnnounce => "waiting for the host's new area to arrive",
        }
    }
}

pub fn rejoin_target(
    last_host_target: Option<PhaseTarget>,
    partner_phase: Option<u16>,
) -> Result<PhaseTarget, RejoinRefusal> {
    let target = last_host_target.ok_or(RejoinRefusal::HostAreaNotKnownYet)?;
    match partner_phase {
        Some(phase) if phase != target.phase => Err(RejoinRefusal::HostMovedSinceLastAnnounce),
        _ => Ok(target),
    }
}
