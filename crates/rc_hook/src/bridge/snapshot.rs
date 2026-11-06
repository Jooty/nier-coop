use std::fmt::Display;

use rc_game::Replicant;
use serde_json::{Value, json};

use crate::memory::InProcessMemory;

pub(super) fn snapshot_values(game: &Replicant<InProcessMemory>) -> Value {
    json!({
        "position": field(game.position().map(|position| {
            json!({"x": position.x, "y": position.y, "z": position.z})
        })),
        "hp": field(game.health()),
        "hp_max": field(game.max_health()),
        "mp": field(game.magic()),
        "mp_max": field(game.max_magic()),
        "level": field(game.level()),
        "experience": field(game.experience()),
        "gold": field(game.gold()),
        "zone": field(game.zone()),
        "phase": field(game.current_phase()),
    })
}

pub(crate) fn field<T: Into<Value>, E: Display>(result: Result<T, E>) -> Value {
    match result {
        Ok(value) => value.into(),
        Err(error) => json!({"error": error.to_string()}),
    }
}
