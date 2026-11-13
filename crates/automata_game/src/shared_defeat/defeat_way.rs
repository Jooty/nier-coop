use serde::{Deserialize, Serialize};

use crate::defeat::{
    DEFEAT_FLOW_CONTINUE_WITHOUT_LEFT_BODY_STATE, DEFEAT_FLOW_FIXED_PHASE_REQUEST_STATE,
    DEFEAT_FLOW_GAME_OVER_MENU_STATE, DEFEAT_FLOW_TITLE_STATE, DEFEAT_FLOW_USUAL_STATE, DefeatFlow,
};

/// The way on the game's flow took after its choosing state: its states 3 to 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DefeatWay {
    RestorePoint,
    RestorePointWithoutBodyScreen,
    GameOverMenu,
    FixedPhaseRequest,
    Title,
}

impl DefeatWay {
    pub fn of_flow(flow: DefeatFlow) -> Option<Self> {
        match flow.state {
            DEFEAT_FLOW_USUAL_STATE => Some(Self::RestorePoint),
            DEFEAT_FLOW_CONTINUE_WITHOUT_LEFT_BODY_STATE => {
                Some(Self::RestorePointWithoutBodyScreen)
            }
            DEFEAT_FLOW_GAME_OVER_MENU_STATE => Some(Self::GameOverMenu),
            DEFEAT_FLOW_FIXED_PHASE_REQUEST_STATE => Some(Self::FixedPhaseRequest),
            DEFEAT_FLOW_TITLE_STATE => Some(Self::Title),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::RestorePoint => "state 3, back to its restore point",
            Self::RestorePointWithoutBodyScreen => {
                "state 4, back to its restore point without the body screen"
            }
            Self::GameOverMenu => "state 5, the game-over menu",
            Self::FixedPhaseRequest => "state 6, the fixed phase request 0xF30",
            Self::Title => "state 7, the title",
        }
    }
}
