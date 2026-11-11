use crate::defeat::{LOWEST_HEALTH_KEPT_ON_THE_PARTNER_BODY, is_hero_defeat_state};

/// The owner's health as the held body shows it here: never below 1, so this game never defeats it.
pub fn health_kept_on_the_partner_body(owner_health: i32, body_max_health: i32) -> i32 {
    owner_health.clamp(
        LOWEST_HEALTH_KEPT_ON_THE_PARTNER_BODY,
        body_max_health.max(LOWEST_HEALTH_KEPT_ON_THE_PARTNER_BODY),
    )
}

/// A defeat state on the held body would run this game's own defeat flow for the other player.
pub fn local_game_may_start_on_the_held_body(state: i32) -> bool {
    !is_hero_defeat_state(state)
}
