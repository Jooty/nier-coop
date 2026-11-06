use crate::stay_together::PhaseTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejoinRefusal {
    HostAreaNotKnownYet,
    HostMovedSinceLastAnnounce,
    HostNotSeenYet,
    NotFreeToAct,
}

impl RejoinRefusal {
    pub fn label(self) -> &'static str {
        match self {
            Self::HostAreaNotKnownYet => "the host has not announced its area yet",
            Self::HostMovedSinceLastAnnounce => "waiting for the host's new area to arrive",
            Self::HostNotSeenYet => "the host's position has not arrived yet",
            Self::NotFreeToAct => "wait until you are free to act (no load, fade or cutscene)",
        }
    }
}

pub fn rejoin_target(
    last_host_target: Option<PhaseTarget>,
    partner_phase: Option<u16>,
    free_to_act: bool,
) -> Result<PhaseTarget, RejoinRefusal> {
    if !free_to_act {
        return Err(RejoinRefusal::NotFreeToAct);
    }
    let target = last_host_target.ok_or(RejoinRefusal::HostAreaNotKnownYet)?;
    match partner_phase {
        None => Err(RejoinRefusal::HostNotSeenYet),
        Some(phase) if phase != target.phase => Err(RejoinRefusal::HostMovedSinceLastAnnounce),
        Some(_) => Ok(target),
    }
}

pub fn host_target_for_session(
    last_host_target: Option<PhaseTarget>,
    connected: bool,
) -> Option<PhaseTarget> {
    last_host_target.filter(|_| connected)
}
