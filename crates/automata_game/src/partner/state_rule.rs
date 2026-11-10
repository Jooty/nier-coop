use serde::{Deserialize, Serialize};

use crate::partner::{
    DIFFERING_MESSAGES_BEFORE_STATE_START, HIGHEST_STATE_ID, NEVER_STARTED_STATES, PartnerDrive,
    STATES_STARTED_ON_THE_BODY,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteStateIds {
    pub state: u16,
    pub variant: u32,
    pub b: u32,
    pub c: u32,
}

/// The arguments of one animation start of the sender's hero, numbered by the sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedStart {
    pub count: u32,
    pub ids: RemoteStateIds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteState {
    pub state: u16,
    pub last_start: Option<RecordedStart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateDecision {
    Nothing,
    Start(RemoteStateIds),
    NotAllowed(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StartKey {
    state: u16,
    count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRule {
    drive: PartnerDrive,
    last_remote: Option<StartKey>,
    last_started: Option<StartKey>,
    differing_messages: u32,
}

impl RecordedStart {
    /// None when the state is not one the game's dispatcher knows.
    pub fn of_call(count: u32, state: i32, variant: u32, b: u32, c: u32) -> Option<Self> {
        let state = u16::try_from(state)
            .ok()
            .filter(|state| *state <= HIGHEST_STATE_ID)?;
        Some(Self {
            count,
            ids: RemoteStateIds {
                state,
                variant,
                b,
                c,
            },
        })
    }
}

impl RemoteState {
    fn start_of_the_live_state(self) -> Option<RecordedStart> {
        self.last_start
            .filter(|start| start.ids.state == self.state)
    }

    fn key(self) -> StartKey {
        StartKey {
            state: self.state,
            count: self.start_of_the_live_state().map(|start| start.count),
        }
    }

    fn start_arguments(self) -> RemoteStateIds {
        // Without a recorded start of this state, phase zero makes the handler start its motion.
        self.start_of_the_live_state().map_or(
            RemoteStateIds {
                state: self.state,
                variant: 0,
                b: 0,
                c: 0,
            },
            |start| start.ids,
        )
    }
}

impl StateRule {
    pub fn new(drive: PartnerDrive) -> Self {
        Self {
            drive,
            last_remote: None,
            last_started: None,
            differing_messages: 0,
        }
    }

    /// Call once for each new message from the other player, with the local body's state id.
    pub fn observe(&mut self, local_state: i32, remote: RemoteState) -> StateDecision {
        let key = remote.key();
        let previous = self.last_remote.replace(key);
        let agrees = i32::from(remote.state) == local_state;
        if agrees || previous != Some(key) {
            self.differing_messages = 0;
        }
        if !agrees {
            self.differing_messages += 1;
        }
        let differed_long_enough = self.differing_messages >= DIFFERING_MESSAGES_BEFORE_STATE_START
            && self.last_started != Some(key);
        let wants_start = match self.drive {
            PartnerDrive::State => {
                let new_start = previous != Some(key);
                let a_recorded_start_is_new =
                    key.count.is_some() && previous.is_some_and(|before| before != key);
                (new_start && (!agrees || a_recorded_start_is_new)) || differed_long_enough
            }
            PartnerDrive::Input => differed_long_enough,
        };
        if !wants_start {
            return StateDecision::Nothing;
        }
        self.differing_messages = 0;
        self.last_started = Some(key);
        if is_started_on_the_body(remote.state) {
            StateDecision::Start(remote.start_arguments())
        } else {
            StateDecision::NotAllowed(remote.state)
        }
    }

    /// Call after a teleport of the body: the game's teleport puts it in its default state, so
    /// the state started last may be started again.
    pub fn forget_started(&mut self) {
        self.last_started = None;
        self.differing_messages = 0;
    }
}

pub fn is_never_started(state: u16) -> bool {
    NEVER_STARTED_STATES.contains(&state)
}

pub fn is_started_on_the_body(state: u16) -> bool {
    STATES_STARTED_ON_THE_BODY.contains(&state) && !is_never_started(state)
}
