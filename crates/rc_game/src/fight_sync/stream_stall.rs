use crate::fight_sync::{STREAM_STALL_LOGGED_EVERY_MS, STREAM_STALL_LOGGED_FROM_MS};

/// The radio manager's state as the game holds it, read for the stall and line start lines.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RadioState {
    /// Bit 2 of the flag word the line start and the radio update test before they abort a line.
    pub abort_flag: bool,
    pub event_scene: u8,
    pub active: u8,
    pub busy: u8,
    pub line_state: i32,
    pub line_time_s: f32,
    pub line_flag: u8,
}

/// What the stall line names: the entry that waits, and what holds the script slot.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamStall<'a> {
    pub seq: u64,
    pub name: &'a str,
    pub waited_ms: u64,
    pub holder: Option<(&'a str, u64)>,
    pub last_yielding_native: Option<(&'static str, u64)>,
    pub radio: Option<RadioState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WaitingEntry {
    seq: u64,
    since_ms: u64,
    next_log_after_ms: u64,
}

/// Watches the next host entry the guest has not run yet: how long it waits, and when a stall
/// line is due (past `STREAM_STALL_LOGGED_FROM_MS`, then once per `STREAM_STALL_LOGGED_EVERY_MS`).
#[derive(Debug, Clone, Default)]
pub struct StreamStallWatch {
    waiting: Option<WaitingEntry>,
}

impl StreamStallWatch {
    pub const fn new() -> Self {
        Self { waiting: None }
    }

    /// `next_waiting`: the seq of the next host entry not run yet, if any.
    pub fn observe(&mut self, next_waiting: Option<u64>, now_ms: u64) {
        let Some(seq) = next_waiting else {
            self.waiting = None;
            return;
        };
        if self.waiting.is_some_and(|waiting| waiting.seq == seq) {
            return;
        }
        self.waiting = Some(WaitingEntry {
            seq,
            since_ms: now_ms,
            next_log_after_ms: STREAM_STALL_LOGGED_FROM_MS,
        });
    }

    pub fn waiting_ms(&self, now_ms: u64) -> Option<u64> {
        self.waiting
            .map(|waiting| now_ms.saturating_sub(waiting.since_ms))
    }

    /// The seq and wait of the entry when a stall line is due now.
    pub fn stall_due(&mut self, now_ms: u64) -> Option<(u64, u64)> {
        let waiting = self.waiting.as_mut()?;
        let waited_ms = now_ms.saturating_sub(waiting.since_ms);
        if waited_ms <= waiting.next_log_after_ms {
            return None;
        }
        waiting.next_log_after_ms = waited_ms + STREAM_STALL_LOGGED_EVERY_MS;
        Some((waiting.seq, waited_ms))
    }
}

pub fn format_stream_stall(stall: &StreamStall<'_>) -> String {
    let holder = match stall.holder {
        Some((name, in_slot_ms)) => format!("{name} (in the slot {in_slot_ms} ms"),
        None => "no known handler (in the slot ? ms".to_owned(),
    };
    let native = match stall.last_yielding_native {
        Some((native, age_ms)) => format!("last yielding native {native} {age_ms} ms ago"),
        None => "no yielding native since the pop".to_owned(),
    };
    let radio = stall.radio.map_or_else(
        || "radio unread".to_owned(),
        |radio| format_radio_state(&radio),
    );
    format!(
        "fight stream: seq {} {} waits behind {holder}, {native}, {radio}), waited {} ms",
        stall.seq, stall.name, stall.waited_ms
    )
}

pub fn format_radio_state(radio: &RadioState) -> String {
    format!(
        "radio active {}, busy {}, line state {}, line time {:.2} s, line flag {}, abort flag {}, event scene {}",
        radio.active,
        radio.busy,
        radio.line_state,
        radio.line_time_s,
        radio.line_flag,
        radio.abort_flag,
        radio.event_scene
    )
}

pub fn format_radio_line_under_menu(handler: &str, radio: &RadioState) -> String {
    format!(
        "radio: a line started with a menu open in {handler}, {}",
        format_radio_state(radio)
    )
}
