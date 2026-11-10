use replicant_proto::StoryCall;

use crate::story_calls::PartnerPlace;
use crate::story_relay::{
    GATHER_RELEASE_REMEMBERED_MS, MAX_SCENE_CALLS_HELD_FOR_PARTNER,
    SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatherAdmission {
    Open,
    JustReleased,
    AlreadyWaiting,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatherAloneReason {
    TimeLimit,
    PartnerNotInPlay,
    ConnectionLost,
    PartnerInAnotherPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatherOutcome {
    PartnerArrived,
    RunAlone(GatherAloneReason),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GatherInputs {
    pub now_ms: u64,
    pub connected: bool,
    pub partner_in_play: bool,
    pub partner_place: PartnerPlace,
    pub local_phase: Option<u16>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GatherRelease {
    pub call: StoryCall,
    pub outcome: GatherOutcome,
    pub waited_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GatherStep {
    pub released: Vec<GatherRelease>,
    pub forgotten: Vec<StoryCall>,
}

#[derive(Debug, Clone, PartialEq)]
struct HeldHostCall {
    call: StoryCall,
    held_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReleasedMark {
    phase: u16,
    released_at_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneGatherHold {
    held: Vec<HeldHostCall>,
    released: Vec<(String, ReleasedMark)>,
}

impl SceneGatherHold {
    pub const fn new() -> Self {
        Self {
            held: Vec::new(),
            released: Vec::new(),
        }
    }

    pub fn is_idle(&self) -> bool {
        self.held.is_empty() && self.released.is_empty()
    }

    pub fn has_room(&self) -> bool {
        self.held.len() < MAX_SCENE_CALLS_HELD_FOR_PARTNER
    }

    pub fn admit(&mut self, phase: u16, function: &str, now_ms: u64) -> GatherAdmission {
        self.released.retain(|(_, mark)| {
            now_ms.saturating_sub(mark.released_at_ms) < GATHER_RELEASE_REMEMBERED_MS
        });
        if let Some(position) = self
            .released
            .iter()
            .position(|(name, mark)| name == function && mark.phase == phase)
        {
            self.released.remove(position);
            return GatherAdmission::JustReleased;
        }
        if self
            .held
            .iter()
            .any(|held| held.call.phase == phase && held.call.function == function)
        {
            return GatherAdmission::AlreadyWaiting;
        }
        if !self.has_room() {
            return GatherAdmission::Full;
        }
        GatherAdmission::Open
    }

    pub fn hold(&mut self, call: StoryCall, now_ms: u64) -> bool {
        let taken = self
            .held
            .iter()
            .any(|held| held.call.phase == call.phase && held.call.function == call.function);
        if taken || !self.has_room() {
            return false;
        }
        self.held.push(HeldHostCall {
            call,
            held_at_ms: now_ms,
        });
        true
    }

    pub fn step(&mut self, inputs: &GatherInputs) -> GatherStep {
        let mut step = GatherStep::default();
        self.released
            .retain(|(_, mark)| Some(mark.phase) == inputs.local_phase);
        let mut still_waiting = Vec::new();
        for held in std::mem::take(&mut self.held) {
            if inputs.local_phase != Some(held.call.phase) {
                step.forgotten.push(held.call);
                continue;
            }
            let waited_ms = inputs.now_ms.saturating_sub(held.held_at_ms);
            let Some(outcome) = outcome_of(inputs, waited_ms) else {
                still_waiting.push(held);
                continue;
            };
            self.released.push((
                held.call.function.clone(),
                ReleasedMark {
                    phase: held.call.phase,
                    released_at_ms: inputs.now_ms,
                },
            ));
            step.released.push(GatherRelease {
                call: held.call,
                outcome,
                waited_ms,
            });
        }
        self.held = still_waiting;
        step
    }
}

fn outcome_of(inputs: &GatherInputs, waited_ms: u64) -> Option<GatherOutcome> {
    if !inputs.connected {
        return Some(GatherOutcome::RunAlone(GatherAloneReason::ConnectionLost));
    }
    if !inputs.partner_in_play {
        return Some(GatherOutcome::RunAlone(GatherAloneReason::PartnerNotInPlay));
    }
    match inputs.partner_place {
        PartnerPlace::Same => return Some(GatherOutcome::PartnerArrived),
        PartnerPlace::OtherPhase => {
            return Some(GatherOutcome::RunAlone(
                GatherAloneReason::PartnerInAnotherPhase,
            ));
        }
        PartnerPlace::TooFar { .. } | PartnerPlace::Unknown => {}
    }
    (waited_ms >= SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS)
        .then_some(GatherOutcome::RunAlone(GatherAloneReason::TimeLimit))
}
