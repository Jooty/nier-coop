use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicI16, AtomicU8, AtomicU16, AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

use rc_game::harness::{pad_button_bit, pad_button_names};
use serde_json::{Value, json};

use crate::bridge::error_reply;
use crate::harness::MAX_HOLD;

const XINPUT_STATE_BYTES: usize = 16;
const THUMB_FULL_SCALE: f64 = 32767.0;
const TRIGGER_FULL_SCALE: f64 = 255.0;
const AXIS_FIELDS: [&str; 4] = ["lx", "ly", "rx", "ry"];
const TRIGGER_FIELDS: [&str; 2] = ["lt", "rt"];

static PAD_OVERRIDE_ACTIVE: AtomicBool = AtomicBool::new(false);
static PAD_BUTTONS: AtomicU16 = AtomicU16::new(0);
static PAD_AXES: [AtomicI16; 4] = [const { AtomicI16::new(0) }; 4];
static PAD_TRIGGERS: [AtomicU8; 2] = [const { AtomicU8::new(0) }; 2];
static PAD_PACKET_NUMBER: AtomicU32 = AtomicU32::new(0);

#[expect(
    unsafe_code,
    reason = "fills the XINPUT_STATE the game passed to XInputGetState"
)]
pub fn write_virtual_pad_state(state_address: usize) -> bool {
    if !PAD_OVERRIDE_ACTIVE.load(Ordering::Relaxed) || state_address == 0 {
        return false;
    }
    let mut bytes = [0_u8; XINPUT_STATE_BYTES];
    bytes[0..4].copy_from_slice(
        &PAD_PACKET_NUMBER
            .fetch_add(1, Ordering::Relaxed)
            .to_le_bytes(),
    );
    bytes[4..6].copy_from_slice(&PAD_BUTTONS.load(Ordering::Relaxed).to_le_bytes());
    bytes[6] = PAD_TRIGGERS[0].load(Ordering::Relaxed);
    bytes[7] = PAD_TRIGGERS[1].load(Ordering::Relaxed);
    for (index, axis) in PAD_AXES.iter().enumerate() {
        let at = 8 + index * 2;
        bytes[at..at + 2].copy_from_slice(&axis.load(Ordering::Relaxed).to_le_bytes());
    }
    // SAFETY: the game passed a writable XINPUT_STATE, which is 16 bytes.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), state_address as *mut u8, XINPUT_STATE_BYTES)
    };
    true
}

pub(super) fn pad_reply(request: &Value) -> Value {
    if request.get("release").and_then(Value::as_bool) == Some(true) {
        clear_pad();
        return json!({"ok": true, "pad": "released"});
    }
    let buttons = match buttons_from(request) {
        Ok(buttons) => buttons,
        Err(message) => return error_reply(&message),
    };
    PAD_BUTTONS.store(buttons, Ordering::Relaxed);
    for (axis, field) in PAD_AXES.iter().zip(AXIS_FIELDS) {
        axis.store(
            scaled(request, field, -1.0, THUMB_FULL_SCALE) as i16,
            Ordering::Relaxed,
        );
    }
    for (trigger, field) in PAD_TRIGGERS.iter().zip(TRIGGER_FIELDS) {
        trigger.store(
            scaled(request, field, 0.0, TRIGGER_FULL_SCALE) as u8,
            Ordering::Relaxed,
        );
    }
    PAD_OVERRIDE_ACTIVE.store(true, Ordering::Relaxed);
    let hold = request
        .get("ms")
        .and_then(Value::as_u64)
        .map(|milliseconds| Duration::from_millis(milliseconds).min(MAX_HOLD));
    if let Some(hold) = hold {
        thread::sleep(hold);
        clear_pad();
    }
    json!({"ok": true, "buttons": buttons, "held_ms": hold.map(|hold| hold.as_millis() as u64)})
}

fn clear_pad() {
    PAD_OVERRIDE_ACTIVE.store(false, Ordering::Relaxed);
    PAD_BUTTONS.store(0, Ordering::Relaxed);
    for axis in &PAD_AXES {
        axis.store(0, Ordering::Relaxed);
    }
    for trigger in &PAD_TRIGGERS {
        trigger.store(0, Ordering::Relaxed);
    }
}

fn buttons_from(request: &Value) -> Result<u16, String> {
    let Some(list) = request.get("buttons").and_then(Value::as_array) else {
        return Ok(0);
    };
    let mut mask = 0_u16;
    for name in list.iter().filter_map(Value::as_str) {
        let bit = pad_button_bit(name).ok_or_else(|| {
            format!(
                "unknown pad button \"{name}\" (known: {:?})",
                pad_button_names()
            )
        })?;
        mask |= bit;
    }
    Ok(mask)
}

fn scaled(request: &Value, field: &str, lowest: f64, full_scale: f64) -> f64 {
    let value = request
        .get(field)
        .and_then(Value::as_f64)
        .unwrap_or(0.0)
        .clamp(lowest, 1.0);
    (value * full_scale).round()
}
