use crate::partner_hud::{
    CHARACTERS_THE_GAME_TEXT_MAY_READ_AS_MARKUP, NAME_OF_AN_UNNAMED_PARTNER, SHOWN_NAME_MAX_CHARS,
    TAG_FADE_END_DISTANCE_UNITS, TAG_FADE_MINIMUM_ALPHA, TAG_FADE_START_DISTANCE_UNITS,
    TAG_PARTNER_STATE_STALE_MS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagInputs {
    pub enabled: bool,
    pub connected: bool,
    pub partner_state_age_ms: Option<u64>,
    pub same_place: bool,
    pub local_in_play: bool,
    pub local_free: bool,
    pub local_paused: bool,
    pub partner_watches_this_player: bool,
}

/// Why the partner's tag and map dot are not drawn now; `None` draws them.
pub fn tag_hidden_reason(inputs: &TagInputs) -> Option<&'static str> {
    if !inputs.enabled {
        return Some("turned off");
    }
    if !inputs.connected {
        return Some("no partner linked");
    }
    let Some(age_ms) = inputs.partner_state_age_ms else {
        return Some("no partner state yet");
    };
    if age_ms > TAG_PARTNER_STATE_STALE_MS {
        return Some("the partner's state is stale");
    }
    if inputs.partner_watches_this_player {
        return Some("the partner watches this game's player");
    }
    if !inputs.local_in_play {
        return Some("this game is not in play");
    }
    if !inputs.same_place {
        return Some("the partner is in another place");
    }
    if inputs.local_paused {
        return Some("this game is paused or in a menu");
    }
    if !inputs.local_free {
        return Some("a scene holds this player");
    }
    None
}

pub fn tag_fade_alpha(distance: f32) -> f32 {
    let span = TAG_FADE_END_DISTANCE_UNITS - TAG_FADE_START_DISTANCE_UNITS;
    let progress = ((distance - TAG_FADE_START_DISTANCE_UNITS) / span).clamp(0.0, 1.0);
    1.0 - progress * (1.0 - TAG_FADE_MINIMUM_ALPHA)
}

pub fn tag_label(name: &str, distance: Option<f32>) -> String {
    match distance {
        Some(distance) => format!("{name} \u{b7} {distance:.0} m"),
        None => name.to_owned(),
    }
}

/// The lobby name as the HUD shows it: no control or markup characters, a bounded length, and a
/// plain word when nothing is left.
pub fn shown_name(lobby_name: &str) -> String {
    let kept: String = lobby_name
        .trim()
        .chars()
        .filter(|character| {
            !character.is_control()
                && !CHARACTERS_THE_GAME_TEXT_MAY_READ_AS_MARKUP.contains(character)
        })
        .take(SHOWN_NAME_MAX_CHARS)
        .collect();
    if kept.trim().is_empty() {
        NAME_OF_AN_UNNAMED_PARTNER.to_owned()
    } else {
        kept
    }
}
