use rc_game::Replicant;
use rc_game::harness::guess_screen;
use serde_json::{Value, json};

use crate::bridge::field;
use crate::harness::settings::injection_enabled;
use crate::memory::InProcessMemory;

pub(super) fn state_reply(game: &Replicant<InProcessMemory>) -> Value {
    let player_present = game.playable_actor().is_ok();
    let map_manager_active = game.map_manager_is_active().unwrap_or(false);
    let screen = guess_screen(player_present, map_manager_active);
    json!({
        "ok": true,
        "harness": injection_enabled(),
        "screen_guess": screen.name(),
        "player_present": player_present,
        "map_manager_active": map_manager_active,
        "phase": field(game.current_phase()),
        "hp": field(game.health()),
        "hp_max": field(game.max_health()),
        "position": field(game.position().map(|position| {
            json!({"x": position.x, "y": position.y, "z": position.z})
        })),
        "event_playing": Value::Null,
    })
}
