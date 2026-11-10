#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockedKillResponse {
    RecoverFromFall,
    GoDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KillGuardInputs {
    pub session_active: bool,
    pub partner_downed: bool,
    pub game_over: bool,
    pub rewinding: bool,
}

pub const fn kill_is_blocked(inputs: KillGuardInputs) -> bool {
    inputs.session_active && !inputs.partner_downed && !inputs.game_over && !inputs.rewinding
}

pub const fn blocked_kill_response(ground_under_local: bool) -> BlockedKillResponse {
    if ground_under_local {
        BlockedKillResponse::GoDown
    } else {
        BlockedKillResponse::RecoverFromFall
    }
}
