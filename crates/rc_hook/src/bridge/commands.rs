use rc_game::build::{PE_HEADER_READ_LEN, PeIdentity};
use rc_game::{GameMemory, Replicant};
use serde_json::{Value, json};

use crate::bridge::phases::{change_phase_reply, phases_reply};
use crate::bridge::snapshot::snapshot_values;
use crate::bridge::{DEFAULT_LOG_LINES, MAX_LOG_LINES, MAX_READ_LEN};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::recent_lines;
use crate::memory::InProcessMemory;

pub(super) struct Bridge {
    build: VerifiedBuild,
    game: Replicant<InProcessMemory>,
}

impl Bridge {
    pub(super) fn new(build: VerifiedBuild) -> Self {
        Self {
            build,
            game: Replicant::new(InProcessMemory::new(&build)),
        }
    }

    pub(super) fn handle_line(&self, line: &str) -> Value {
        let request: Value = match serde_json::from_str(line) {
            Ok(request) => request,
            Err(error) => return error_reply(&format!("invalid JSON ({error})")),
        };
        let Some(command) = request.get("cmd").and_then(Value::as_str) else {
            return error_reply("missing string field \"cmd\"");
        };
        match command {
            "ping" => json!({"ok": true, "pong": true}),
            "build" => self.build_reply(),
            "read" => self.read_reply(&request),
            "snapshot" => json!({"ok": true, "values": snapshot_values(&self.game)}),
            "phases" => phases_reply(&self.game),
            "change_phase" => change_phase_reply(&request),
            "log" => log_reply(&request),
            other => error_reply(&format!("unknown command \"{other}\"")),
        }
    }

    fn build_reply(&self) -> Value {
        let base = self.build.module_base();
        let identity = self
            .game
            .memory()
            .read_array::<PE_HEADER_READ_LEN>(base)
            .ok()
            .and_then(|header| PeIdentity::parse(&header).ok());
        json!({
            "ok": true,
            "gate": "pass",
            "module_base": format!("{base:#x}"),
            "size_of_image": identity.map(|identity| identity.size_of_image),
            "time_date_stamp": identity.map(|identity| identity.time_date_stamp),
        })
    }

    fn read_reply(&self, request: &Value) -> Value {
        let Some(address) = request.get("address").and_then(parse_number) else {
            return error_reply("missing or invalid \"address\"");
        };
        let Some(length) = request
            .get("len")
            .and_then(parse_number)
            .and_then(|length| usize::try_from(length).ok())
            .filter(|length| (1..=MAX_READ_LEN).contains(length))
        else {
            return error_reply(&format!("\"len\" must be 1 to {MAX_READ_LEN}"));
        };
        let is_rva = request.get("rva").and_then(Value::as_bool).unwrap_or(false);
        let absolute = if is_rva {
            address.checked_add(self.build.module_base())
        } else {
            Some(address)
        };
        let Some(absolute) = absolute else {
            return error_reply("address overflows");
        };
        let mut buffer = vec![0_u8; length];
        match self.game.memory().read(absolute, &mut buffer) {
            Ok(()) => json!({
                "ok": true,
                "address": format!("{absolute:#x}"),
                "len": length,
                "hex": hex_string(&buffer),
            }),
            Err(error) => error_reply(&error.to_string()),
        }
    }
}

fn log_reply(request: &Value) -> Value {
    let count = request
        .get("n")
        .and_then(Value::as_u64)
        .and_then(|count| usize::try_from(count).ok())
        .unwrap_or(DEFAULT_LOG_LINES)
        .min(MAX_LOG_LINES);
    json!({"ok": true, "lines": recent_lines(count)})
}

pub(super) fn error_reply(message: &str) -> Value {
    json!({"ok": false, "error": message})
}

fn parse_number(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => {
            let text = text.trim();
            match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
                Some(digits) => u64::from_str_radix(digits, 16).ok(),
                None => text.parse().ok(),
            }
        }
        _ => None,
    }
}

fn hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
