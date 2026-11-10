#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatherFacts {
    pub same_phase: bool,
    pub guest_in_side_view: bool,
    pub host_in_side_view: bool,
    pub in_own_talk: bool,
    pub in_event_scene: bool,
    pub downed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatherStayReason {
    OtherPhase,
    GuestInSideView,
    HostInSideView,
    InOwnTalk,
    InOwnScene,
    Downed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatherVerdict {
    MoveToHost,
    Stay(GatherStayReason),
}

impl GatherStayReason {
    pub fn describe(self) -> &'static str {
        match self {
            Self::OtherPhase => "you are in another area",
            Self::GuestInSideView => "you are in a side view",
            Self::HostInSideView => "the host is in a side view",
            Self::InOwnTalk => "you are in a talk",
            Self::InOwnScene => "you are in a scene",
            Self::Downed => "you are downed",
        }
    }
}

pub fn gather_verdict(facts: &GatherFacts) -> GatherVerdict {
    let reason = if !facts.same_phase {
        GatherStayReason::OtherPhase
    } else if facts.guest_in_side_view {
        GatherStayReason::GuestInSideView
    } else if facts.host_in_side_view {
        GatherStayReason::HostInSideView
    } else if facts.in_own_talk {
        GatherStayReason::InOwnTalk
    } else if facts.in_event_scene {
        GatherStayReason::InOwnScene
    } else if facts.downed {
        GatherStayReason::Downed
    } else {
        return GatherVerdict::MoveToHost;
    };
    GatherVerdict::Stay(reason)
}
