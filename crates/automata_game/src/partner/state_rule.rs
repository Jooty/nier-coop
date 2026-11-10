use crate::partner::{DIFFERING_MESSAGES_BEFORE_STATE_START, NEVER_STARTED_STATES, PartnerDrive};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteStateIds {
    pub state: u16,
    pub variant: u32,
    pub b: u32,
    pub c: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateDecision {
    Nothing,
    Start(RemoteStateIds),
    Blocked(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRule {
    drive: PartnerDrive,
    last_remote: Option<RemoteStateIds>,
    last_started: Option<RemoteStateIds>,
    differing_messages: u32,
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
    pub fn observe(&mut self, local_state: i32, remote: RemoteStateIds) -> StateDecision {
        let previous = self.last_remote.replace(remote);
        let agrees = i32::from(remote.state) == local_state;
        let wants_start = match self.drive {
            PartnerDrive::State => {
                let changed = previous != Some(remote);
                let same_state_new_variant =
                    changed && previous.is_some_and(|before| before.state == remote.state);
                changed && (!agrees || same_state_new_variant)
            }
            PartnerDrive::Input => {
                if agrees {
                    self.differing_messages = 0;
                    return StateDecision::Nothing;
                }
                if previous != Some(remote) {
                    self.differing_messages = 0;
                }
                self.differing_messages += 1;
                self.differing_messages >= DIFFERING_MESSAGES_BEFORE_STATE_START
                    && self.last_started != Some(remote)
            }
        };
        if !wants_start {
            return StateDecision::Nothing;
        }
        self.differing_messages = 0;
        self.last_started = Some(remote);
        if is_never_started(remote.state) {
            StateDecision::Blocked(remote.state)
        } else {
            StateDecision::Start(remote)
        }
    }
}

pub fn is_never_started(state: u16) -> bool {
    NEVER_STARTED_STATES.contains(&state)
}
