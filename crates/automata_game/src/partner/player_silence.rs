use crate::partner::PARTNER_SILENCE_BEFORE_GIVE_BACK_MS;

/// A player that has not sent a state yet counts as in play, so the hello still reaches it.
pub fn player_is_in_play(newest_state_age_ms: Option<u64>) -> bool {
    newest_state_age_ms.is_none_or(|age| age <= PARTNER_SILENCE_BEFORE_GIVE_BACK_MS)
}
