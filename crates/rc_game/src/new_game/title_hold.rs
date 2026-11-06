use rc_proto::GameReadiness;

use crate::new_game::TITLE_HOLD_TIMEOUT_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitleHold {
    Hold,
    Release,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TitleHoldInputs {
    pub waited_ms: u64,
    pub launcher_linked: bool,
    pub partner_readiness: GameReadiness,
}

pub fn title_hold(inputs: TitleHoldInputs) -> TitleHold {
    if inputs.waited_ms >= TITLE_HOLD_TIMEOUT_MS {
        return TitleHold::TimedOut;
    }
    let waiting_for_launcher = !inputs.launcher_linked;
    let partner_game_not_open = inputs.partner_readiness == GameReadiness::NotLinked;
    if waiting_for_launcher || partner_game_not_open {
        TitleHold::Hold
    } else {
        TitleHold::Release
    }
}
