use std::thread;
use std::time::Duration;

use serde_json::{Value, json};

use crate::bridge::error_reply;
use crate::harness::injected_input::set_mouse_button;
use crate::harness::window_messages::{game_window_handle, move_pointer, post_mouse_button};
use crate::harness::{MAX_HOLD, POINTER_SETTLE};

const DEFAULT_CLICK: Duration = Duration::from_millis(80);

pub(super) fn mouse_reply(request: &Value, click: bool) -> Value {
    let Some(window) = game_window_handle() else {
        return error_reply("the game window has not been seen yet");
    };
    let (Some(x), Some(y)) = (coordinate(request, "x"), coordinate(request, "y")) else {
        return error_reply("\"x\" and \"y\" (client pixels) are required integers");
    };
    let button = request
        .get("button")
        .and_then(Value::as_str)
        .unwrap_or("left");
    let index = match button {
        "left" => 0,
        "right" => 1,
        other => return error_reply(&format!("unknown mouse button \"{other}\"")),
    };
    move_pointer(window, x, y);
    if !click {
        return json!({"ok": true, "moved_to": [x, y]});
    }
    let hold = request
        .get("ms")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_CLICK, Duration::from_millis)
        .min(MAX_HOLD);
    thread::sleep(POINTER_SETTLE);
    set_mouse_button(index, true);
    post_mouse_button(index, true, x, y);
    thread::sleep(hold);
    set_mouse_button(index, false);
    post_mouse_button(index, false, x, y);
    json!({"ok": true, "clicked": button, "at": [x, y], "hold_ms": hold.as_millis() as u64})
}

fn coordinate(request: &Value, field: &str) -> Option<i32> {
    request
        .get(field)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}
