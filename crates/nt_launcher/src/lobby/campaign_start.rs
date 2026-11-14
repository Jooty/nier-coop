use crate::game_link::{StartChoice, StartSetting};
use crate::net::LOBBY_MODE_CONTINUE;

/// The start the game is told: the lobby's New game or Continue for a game whose hook runs the
/// campaign start itself, the configured start otherwise (free roam and the other game), on the
/// host's map when the mode starts on one.
pub fn launch_start(
    configured: StartSetting,
    hook_runs_the_campaign_start: bool,
    lobby_mode: u8,
    campaign: bool,
    map_start_index: Option<u8>,
) -> StartSetting {
    let configured = match map_start_index {
        Some(index) => StartSetting {
            index,
            ..configured
        },
        None => configured,
    };
    if !hook_runs_the_campaign_start {
        return configured;
    }
    let choice = if lobby_mode == LOBBY_MODE_CONTINUE {
        StartChoice::Continue
    } else if campaign {
        StartChoice::NewGame
    } else {
        return configured;
    };
    StartSetting {
        choice,
        index: 0,
        slot: 0,
        // A campaign plays the story; holding it would refuse the opening's own scene changes.
        story_allowed: true,
    }
}
