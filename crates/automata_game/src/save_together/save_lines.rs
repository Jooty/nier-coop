pub fn host_told_line(number: u32) -> String {
    format!("save together: told the guest that this game saved (host save {number})")
}

pub fn host_not_told_line(reason: &str) -> String {
    format!("save together: this game saved, not told: {reason}")
}

pub fn guest_heard_line(number: u32, replaced: Option<u32>) -> String {
    match replaced {
        Some(old) => format!(
            "save together: heard host save {number}, it replaces host save {old} that was still waiting"
        ),
        None => format!("save together: heard host save {number}"),
    }
}

pub fn guest_waits_line(number: u32, waited_ms: u64, reason: &str) -> String {
    format!("save together: host save {number} waits for {waited_ms} ms: {reason}")
}

pub fn guest_asks_line(number: u32, waited_ms: u64) -> String {
    format!(
        "save together: asking the save manager to save, for host save {number} (waited {waited_ms} ms)"
    )
}

pub fn guest_asked_slot_line(slot: u32) -> String {
    format!("save together: the save goes to slot {slot} of this game")
}

pub fn guest_saved_line(number: u32, took_ms: u64) -> String {
    format!("save together: saved for host save {number} after {took_ms} ms")
}

pub fn guest_save_failed_line(number: u32, took_ms: u64) -> String {
    format!(
        "save together: the save for host save {number} failed after {took_ms} ms, play goes on"
    )
}

pub fn guest_save_not_taken_line(number: u32, waited_ms: u64) -> String {
    format!(
        "save together: the save manager never went busy for host save {number} ({waited_ms} ms), play goes on"
    )
}

pub fn guest_save_timed_out_line(number: u32, took_ms: u64) -> String {
    format!(
        "save together: the save for host save {number} was still running after {took_ms} ms, play goes on"
    )
}

pub fn guest_gave_up_line(number: u32, waited_ms: u64, reason: &str) -> String {
    format!(
        "save together: gave up on host save {number} after {waited_ms} ms ({reason}), this game does not save for it"
    )
}

pub fn guest_dropped_line(number: u32, reason: &str) -> String {
    format!("save together: host save {number} dropped, {reason}")
}
