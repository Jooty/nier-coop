use std::thread;
use std::time::Duration;

use rc_game::harness::{KeyCode, lookup_key};
use serde_json::{Value, json};

use crate::bridge::error_reply;
use crate::harness::injected_input::{release_all_keys, set_key};
use crate::harness::window_messages::post_key;
use crate::harness::{DEFAULT_TAP, MAX_HOLD};

pub(super) fn key_reply(request: &Value) -> Value {
    let action = request
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("tap");
    if action == "release_all" {
        release_all_keys();
        return json!({"ok": true, "released": "all"});
    }
    let keys = match keys_from(request) {
        Ok(keys) => keys,
        Err(message) => return error_reply(&message),
    };
    let hold = match hold_from(request) {
        Ok(hold) => hold,
        Err(message) => return error_reply(&message),
    };
    match action {
        "press" => set_all(&keys, true),
        "release" => set_all(&keys, false),
        "tap" => {
            set_all(&keys, true);
            thread::sleep(hold);
            set_all(&keys, false);
        }
        other => return error_reply(&format!("unknown key action \"{other}\"")),
    }
    let names: Vec<&str> = keys.iter().map(|key| key.name).collect();
    json!({"ok": true, "action": action, "keys": names, "hold_ms": hold.as_millis() as u64})
}

fn keys_from(request: &Value) -> Result<Vec<KeyCode>, String> {
    let mut names: Vec<&str> = Vec::new();
    if let Some(name) = request.get("key").and_then(Value::as_str) {
        names.push(name);
    }
    if let Some(list) = request.get("keys").and_then(Value::as_array) {
        names.extend(list.iter().filter_map(Value::as_str));
    }
    if names.is_empty() {
        return Err("missing \"key\" (a name) or \"keys\" (a list of names)".to_owned());
    }
    names
        .into_iter()
        .map(|name| lookup_key(name).ok_or_else(|| format!("unknown key \"{name}\"")))
        .collect()
}

fn hold_from(request: &Value) -> Result<Duration, String> {
    let Some(value) = request.get("ms") else {
        return Ok(DEFAULT_TAP);
    };
    let milliseconds = value
        .as_u64()
        .ok_or_else(|| "\"ms\" must be a non-negative integer".to_owned())?;
    Ok(Duration::from_millis(milliseconds).min(MAX_HOLD))
}

fn set_all(keys: &[KeyCode], down: bool) {
    for key in keys {
        set_key(key.direct_input_scan_code, down);
        post_key(key.virtual_key, key.direct_input_scan_code, down);
    }
}
