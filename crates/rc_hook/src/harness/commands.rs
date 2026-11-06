use rc_game::Replicant;
use serde_json::Value;

use crate::bridge::error_reply;
use crate::harness::key_command::key_reply;
use crate::harness::mouse_click::mouse_reply;
use crate::harness::pad::pad_reply;
use crate::harness::quit::quit_reply;
use crate::harness::screenshot::screenshot_reply;
use crate::harness::settings::injection_enabled;
use crate::harness::state::state_reply;
use crate::memory::InProcessMemory;

pub fn reply(command: &str, request: &Value, game: &Replicant<InProcessMemory>) -> Value {
    if command == "state" {
        return state_reply(game);
    }
    if !injection_enabled() {
        return error_reply("harness mode is off: set harness = true in replicant-coop.toml");
    }
    match command {
        "key" => key_reply(request),
        "mouse_click" => mouse_reply(request, true),
        "mouse_move" => mouse_reply(request, false),
        "pad" => pad_reply(request),
        "screenshot" => screenshot_reply(request),
        "quit" => quit_reply(request),
        other => error_reply(&format!("unknown harness command \"{other}\"")),
    }
}
