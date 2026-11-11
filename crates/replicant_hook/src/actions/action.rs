use replicant_game::PhaseEntry;
use replicant_game::offsets::MAX_PHASE_POINT;
use replicant_proto::StoryJumpPoint;

use crate::actions::ActionError;
use crate::actions::MAX_GOLD_PER_ACTION;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ChangePhase {
        phase: &'static PhaseEntry,
        point: i32,
    },
    SetInfiniteHealth {
        enabled: bool,
    },
    SetInfiniteMagic {
        enabled: bool,
    },
    SetNoclip {
        enabled: bool,
    },
    StoryJump(StoryJumpPoint),
    BringGuestHere,
    GoToGuest,
    RefillHealth,
    RefillMagic,
    AddGold {
        amount: i32,
    },
}

impl Action {
    pub fn change_phase(name: &str, point: i32) -> Result<Self, ActionError> {
        let phase =
            PhaseEntry::find(name).ok_or_else(|| ActionError::UnknownPhase(name.to_owned()))?;
        if !(0..=MAX_PHASE_POINT).contains(&point) {
            return Err(ActionError::PointOutOfRange { point });
        }
        Ok(Self::ChangePhase { phase, point })
    }

    pub fn add_gold(amount: i32) -> Result<Self, ActionError> {
        if !(1..=MAX_GOLD_PER_ACTION).contains(&amount) {
            return Err(ActionError::GoldOutOfRange { amount });
        }
        Ok(Self::AddGold { amount })
    }
}
