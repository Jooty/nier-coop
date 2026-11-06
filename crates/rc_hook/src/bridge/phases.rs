use rc_game::{PHASE_TABLE, Replicant};
use serde_json::{Value, json};

use crate::actions::{self, Action};
use crate::bridge::commands::error_reply;
use crate::bridge::snapshot::field;
use crate::memory::InProcessMemory;

pub(super) fn phases_reply(game: &Replicant<InProcessMemory>) -> Value {
    let phases: Vec<Value> = PHASE_TABLE
        .iter()
        .map(|entry| json!({"name": entry.name, "area": entry.area}))
        .collect();
    json!({
        "ok": true,
        "current": field(game.current_phase()),
        "map_manager_active": game.map_manager_is_active().ok(),
        "queue_is_draining": actions::is_draining(),
        "phases": phases,
    })
}

pub(super) fn change_phase_reply(request: &Value) -> Value {
    let Some(name) = request.get("phase").and_then(Value::as_str) else {
        return error_reply("missing string field \"phase\"");
    };
    let Some(point) = point_from(request) else {
        return error_reply("\"point\" must be an integer");
    };
    if !actions::is_draining() {
        return error_reply("the game-thread hook is not installed, nothing would run the action");
    }
    let action = match Action::change_phase(name, point) {
        Ok(action) => action,
        Err(error) => return error_reply(&error.to_string()),
    };
    match actions::enqueue(action) {
        Ok(()) => json!({"ok": true, "queued": true, "phase": name, "point": point}),
        Err(error) => error_reply(&error.to_string()),
    }
}

fn point_from(request: &Value) -> Option<i32> {
    match request.get("point") {
        None => Some(0),
        Some(value) => value.as_i64().and_then(|point| i32::try_from(point).ok()),
    }
}
