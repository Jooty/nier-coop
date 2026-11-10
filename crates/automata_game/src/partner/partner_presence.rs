use crate::partner::PARTNER_SILENCE_BEFORE_GIVE_BACK_MS;

/// True while the other player's game sends states: connected launchers alone are not enough.
pub fn partner_is_in_play(connected: bool, newest_state_age_ms: Option<u64>) -> bool {
    connected && newest_state_age_ms.is_some_and(|age| age <= PARTNER_SILENCE_BEFORE_GIVE_BACK_MS)
}
