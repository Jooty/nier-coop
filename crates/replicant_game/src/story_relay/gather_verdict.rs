use replicant_proto::GatherReply;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GatherFacts {
    pub same_phase: bool,
    pub in_side_view: bool,
    pub starter_in_side_view: bool,
    pub in_own_talk: bool,
    pub in_event_scene: bool,
    pub downed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatherStayReason {
    OtherPhase,
    InOwnTalk,
    InOwnScene,
    Downed,
    SideViewNotGiven,
    DidNotArrive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatherVerdict {
    Come { across_side_view: bool },
    Stay(GatherStayReason),
}

impl GatherStayReason {
    pub fn describe(self) -> &'static str {
        match self {
            Self::OtherPhase => "you are in another area",
            Self::InOwnTalk => "you are in a talk",
            Self::InOwnScene => "you are in a scene",
            Self::Downed => "you are downed",
            Self::SideViewNotGiven => "your game did not give the partner's camera view there",
            Self::DidNotArrive => "your game did not take the move",
        }
    }

    pub fn reply(self) -> GatherReply {
        match self {
            Self::OtherPhase => GatherReply::InAnotherArea,
            Self::InOwnTalk => GatherReply::InOwnTalk,
            Self::InOwnScene => GatherReply::InOwnScene,
            Self::Downed => GatherReply::Downed,
            Self::SideViewNotGiven => GatherReply::SideViewNotGiven,
            Self::DidNotArrive => GatherReply::DidNotArrive,
        }
    }
}

impl GatherVerdict {
    pub fn comes(self) -> bool {
        matches!(self, Self::Come { .. })
    }
}

pub fn gather_verdict(facts: &GatherFacts) -> GatherVerdict {
    let reason = if !facts.same_phase {
        GatherStayReason::OtherPhase
    } else if facts.in_own_talk {
        GatherStayReason::InOwnTalk
    } else if facts.in_event_scene {
        GatherStayReason::InOwnScene
    } else if facts.downed {
        GatherStayReason::Downed
    } else {
        return GatherVerdict::Come {
            across_side_view: facts.in_side_view != facts.starter_in_side_view,
        };
    };
    GatherVerdict::Stay(reason)
}

pub fn describe_partner_reply(reply: GatherReply) -> &'static str {
    match reply {
        GatherReply::Ready => "the partner is ready",
        GatherReply::InAnotherArea => "the partner is in another area",
        GatherReply::InOwnTalk => "the partner is in a talk of their own, which is kept",
        GatherReply::InOwnScene => "the partner is in a scene of their own",
        GatherReply::Downed => "the partner is downed",
        GatherReply::SideViewNotGiven => {
            "the partner's game did not give your camera view at your spot, the partner went back"
        }
        GatherReply::DidNotArrive => "the partner's game did not take the move",
    }
}
