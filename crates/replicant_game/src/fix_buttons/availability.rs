use crate::stay_together::FollowRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixButton {
    GoToPartner,
    ResyncStory,
    RejoinHostArea,
    RespawnPartner,
    CopyLogs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonFacts {
    pub role: FollowRole,
    pub connected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixButtonState {
    Enabled,
    Disabled(&'static str),
}

impl FixButton {
    pub fn log_id(self) -> &'static str {
        match self {
            Self::GoToPartner => "go_to_partner",
            Self::ResyncStory => "resync_story",
            Self::RejoinHostArea => "rejoin_host_area",
            Self::RespawnPartner => "respawn_partner",
            Self::CopyLogs => "copy_logs",
        }
    }

    pub fn state(self, facts: ButtonFacts) -> FixButtonState {
        match self {
            Self::CopyLogs => FixButtonState::Enabled,
            Self::GoToPartner | Self::RespawnPartner if !facts.connected => {
                FixButtonState::Disabled("needs a connected partner")
            }
            Self::ResyncStory | Self::RejoinHostArea if !facts.connected => {
                FixButtonState::Disabled("needs a connected host")
            }
            Self::ResyncStory | Self::RejoinHostArea if facts.role != FollowRole::Guest => {
                FixButtonState::Disabled("guest only")
            }
            _ => FixButtonState::Enabled,
        }
    }
}
