use crate::story_calls::PartnerPlace;
use crate::story_relay::GatherAdmission;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenePullFacts {
    pub starts_scene: bool,
    pub led_by_partner: bool,
    pub partner_place: PartnerPlace,
    pub admission: GatherAdmission,
    pub relay_queue_has_room: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenePullSkip {
    NotASceneStart,
    LedByPartner,
    PartnerHere,
    PartnerInAnotherArea,
    PartnerPlaceUnknown,
    ReleasedFromTheWait,
    NoRoomToWait,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScenePull {
    WaitAndAskPartnerOver { units: f32 },
    DropRepeatedFire,
    RunNow(ScenePullSkip),
}

impl ScenePullSkip {
    pub fn is_worth_a_log_line(self) -> bool {
        matches!(
            self,
            Self::PartnerInAnotherArea | Self::PartnerPlaceUnknown | Self::NoRoomToWait
        )
    }

    pub fn describe(self) -> &'static str {
        match self {
            Self::NotASceneStart => "it is not a scene start",
            Self::LedByPartner => "the partner started it",
            Self::PartnerHere => "the partner is here",
            Self::PartnerInAnotherArea => "the partner is in another area",
            Self::PartnerPlaceUnknown => "the partner's place is not known",
            Self::ReleasedFromTheWait => "its wait for the partner is over",
            Self::NoRoomToWait => "no room to hold it",
        }
    }
}

/// What a player's own scene start does before it runs: with the partner elsewhere in the same
/// area it waits a bounded time while the partner is asked over; in every other case it runs now.
pub fn scene_pull(facts: &ScenePullFacts) -> ScenePull {
    if !facts.starts_scene {
        return ScenePull::RunNow(ScenePullSkip::NotASceneStart);
    }
    if facts.led_by_partner {
        return ScenePull::RunNow(ScenePullSkip::LedByPartner);
    }
    match facts.admission {
        GatherAdmission::JustReleased => {
            return ScenePull::RunNow(ScenePullSkip::ReleasedFromTheWait);
        }
        GatherAdmission::AlreadyWaiting => return ScenePull::DropRepeatedFire,
        GatherAdmission::Open | GatherAdmission::Full => {}
    }
    let units = match facts.partner_place {
        PartnerPlace::Same => return ScenePull::RunNow(ScenePullSkip::PartnerHere),
        PartnerPlace::OtherPhase => {
            return ScenePull::RunNow(ScenePullSkip::PartnerInAnotherArea);
        }
        PartnerPlace::Unknown => return ScenePull::RunNow(ScenePullSkip::PartnerPlaceUnknown),
        PartnerPlace::TooFar { units } => units,
    };
    if facts.admission == GatherAdmission::Full || !facts.relay_queue_has_room {
        return ScenePull::RunNow(ScenePullSkip::NoRoomToWait);
    }
    ScenePull::WaitAndAskPartnerOver { units }
}
