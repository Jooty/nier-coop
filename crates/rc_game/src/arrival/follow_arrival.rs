use crate::arrival::{
    ARRIVAL_ACCEPTABLE_DISTANCE, ARRIVAL_GIVE_UP_MS, ARRIVAL_RETRY_MS, ARRIVAL_SETTLE_MS,
};
use crate::stay_together::PhaseTarget;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceReason {
    TooFarFromHost,
    NoGroundUnderYou,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrivalStep {
    Idle,
    Waiting,
    Place(PlaceReason),
    AlreadyBesideHost,
    GaveUp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrivalInputs {
    pub now_ms: u64,
    pub local_phase: Option<u16>,
    pub partner_phase: Option<u16>,
    pub free_to_act: bool,
    pub distance_to_partner: Option<f32>,
    pub ground_under_local: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArrivalGuard {
    armed: Option<Armed>,
    ready_since_ms: Option<u64>,
    retry_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Armed {
    target: PhaseTarget,
    deadline_ms: u64,
}

impl ArrivalGuard {
    pub const fn new() -> Self {
        Self {
            armed: None,
            ready_since_ms: None,
            retry_at_ms: 0,
        }
    }

    pub fn arm(&mut self, target: PhaseTarget, now_ms: u64) {
        self.armed = Some(Armed {
            target,
            deadline_ms: now_ms + ARRIVAL_GIVE_UP_MS,
        });
        self.ready_since_ms = None;
        self.retry_at_ms = 0;
    }

    pub fn is_armed(&self) -> bool {
        self.armed.is_some()
    }

    pub fn placed(&mut self) {
        self.disarm();
    }

    pub fn step(&mut self, inputs: &ArrivalInputs) -> ArrivalStep {
        let Some(armed) = self.armed else {
            return ArrivalStep::Idle;
        };
        if inputs.now_ms >= armed.deadline_ms {
            self.disarm();
            return ArrivalStep::GaveUp;
        }
        let both_loaded = inputs.local_phase == Some(armed.target.phase)
            && inputs.partner_phase == Some(armed.target.phase);
        if !(both_loaded && inputs.free_to_act) {
            self.ready_since_ms = None;
            return ArrivalStep::Waiting;
        }
        let ready_since = *self.ready_since_ms.get_or_insert(inputs.now_ms);
        if inputs.now_ms < ready_since + ARRIVAL_SETTLE_MS || inputs.now_ms < self.retry_at_ms {
            return ArrivalStep::Waiting;
        }
        let reason = if !inputs.ground_under_local {
            Some(PlaceReason::NoGroundUnderYou)
        } else if inputs
            .distance_to_partner
            .is_none_or(|distance| distance > ARRIVAL_ACCEPTABLE_DISTANCE)
        {
            Some(PlaceReason::TooFarFromHost)
        } else {
            None
        };
        match reason {
            Some(reason) => {
                self.retry_at_ms = inputs.now_ms + ARRIVAL_RETRY_MS;
                ArrivalStep::Place(reason)
            }
            None => {
                self.disarm();
                ArrivalStep::AlreadyBesideHost
            }
        }
    }

    fn disarm(&mut self) {
        *self = Self::new();
    }
}
