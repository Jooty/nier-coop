use serde_json::{Value, json};

use crate::actions::{self, Action};
use crate::bridge::commands::error_reply;
use crate::puppet::{self, PuppetInfo};

pub(super) fn puppet_reply() -> Value {
    let puppets: Vec<Value> = puppet::snapshot().iter().map(puppet_json).collect();
    json!({
        "ok": true,
        "queue_is_draining": actions::is_draining(),
        "mirror_delay_ms": puppet::mirror_delay_ms(),
        "puppets": puppets,
    })
}

pub(super) fn puppet_spawn_reply(request: &Value) -> Value {
    let delay_ms = match request.get("delay_ms") {
        None => i32::from(puppet::mirror_delay_ms()),
        Some(value) => match value.as_i64().and_then(|delay| i32::try_from(delay).ok()) {
            Some(delay) => delay,
            None => return error_reply("\"delay_ms\" must be an integer"),
        },
    };
    queue_reply(
        Action::puppet_spawn(delay_ms),
        json!({"delay_ms": delay_ms}),
    )
}

pub(super) fn puppet_despawn_reply(request: &Value) -> Value {
    let id = match request.get("id") {
        None => None,
        Some(value) => match value.as_u64().and_then(|id| u32::try_from(id).ok()) {
            Some(id) => Some(id),
            None => return error_reply("\"id\" must be a puppet id"),
        },
    };
    queue_reply(Ok(Action::PuppetDespawn { id }), json!({"id": id}))
}

fn queue_reply(action: Result<Action, actions::ActionError>, echo: Value) -> Value {
    if !actions::is_draining() {
        return error_reply("the game-thread hook is not installed, nothing would run the action");
    }
    let action = match action {
        Ok(action) => action,
        Err(error) => return error_reply(&error.to_string()),
    };
    match actions::enqueue(action) {
        Ok(()) => json!({"ok": true, "queued": true, "request": echo}),
        Err(error) => error_reply(&error.to_string()),
    }
}

fn puppet_json(info: &PuppetInfo) -> Value {
    json!({
        "id": info.id,
        "actor": format!("{:#x}", info.actor),
        "spawned_in_phase": info.spawned_in_phase,
        "delay_ms": info.delay_ms,
        "liveness_is_trusted": info.liveness_is_trusted,
        "position": info.position,
        "yaw": info.yaw,
        "motion_index": info.motion_index,
        "motion_name": info.motion_name,
        "frame": info.frame,
        "last_error": info.last_error,
    })
}
