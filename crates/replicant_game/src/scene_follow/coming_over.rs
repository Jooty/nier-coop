use crate::story_calls::PartnerPlace;
use crate::story_relay::{GatherStayReason, GatherVerdict, SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnnounceVerdict {
    FollowNow,
    ComeOver { move_needed: bool },
    AlreadyComing,
    Drop(AnnounceDrop),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnounceDrop {
    HostNotFarAway,
    Stay(GatherStayReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComingStep {
    Idle,
    Waiting,
    Arrived {
        waited_ms: u64,
        callbacks: Vec<String>,
    },
    GaveUp {
        waited_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Coming {
    callbacks: Vec<String>,
    started_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ComingOver {
    coming: Option<Coming>,
}

impl ComingOver {
    pub const fn new() -> Self {
        Self { coming: None }
    }

    pub fn reset(&mut self) {
        self.coming = None;
    }

    pub fn is_coming(&self) -> bool {
        self.coming.is_some()
    }

    pub fn on_announce(
        &mut self,
        place: PartnerPlace,
        verdict: GatherVerdict,
        moved_to_the_host_just_now: bool,
        callbacks: Vec<String>,
        now_ms: u64,
    ) -> AnnounceVerdict {
        if place.is_same() {
            return AnnounceVerdict::FollowNow;
        }
        if self.coming.is_some() {
            return AnnounceVerdict::AlreadyComing;
        }
        if !matches!(place, PartnerPlace::TooFar { .. }) {
            return AnnounceVerdict::Drop(AnnounceDrop::HostNotFarAway);
        }
        if let GatherVerdict::Stay(reason) = verdict {
            return AnnounceVerdict::Drop(AnnounceDrop::Stay(reason));
        }
        self.coming = Some(Coming {
            callbacks,
            started_ms: now_ms,
        });
        AnnounceVerdict::ComeOver {
            move_needed: !moved_to_the_host_just_now,
        }
    }

    pub fn step(&mut self, place: PartnerPlace, settled_there: bool, now_ms: u64) -> ComingStep {
        let Some(coming) = &self.coming else {
            return ComingStep::Idle;
        };
        let waited_ms = now_ms.saturating_sub(coming.started_ms);
        if place.is_same() && settled_there {
            let callbacks = std::mem::take(&mut self.coming).map_or_else(Vec::new, |c| c.callbacks);
            return ComingStep::Arrived {
                waited_ms,
                callbacks,
            };
        }
        if waited_ms >= SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS {
            self.coming = None;
            return ComingStep::GaveUp { waited_ms };
        }
        ComingStep::Waiting
    }
}
