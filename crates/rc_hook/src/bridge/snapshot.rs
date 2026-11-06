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
        "mp": field(game.magic()),
        "gold": field(game.gold()),
        "zone": field(game.zone()),
    })
}

fn field<T: Into<Value>, E: Display>(result: Result<T, E>) -> Value {
    match result {
        Ok(value) => value.into(),
        Err(error) => json!({"error": error.to_string()}),
    }
}
