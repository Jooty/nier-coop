use crate::memory::GameMemory;
use crate::move_log::{
    RATE_TABLE_CHANNEL_STRIDE, RATE_TABLE_ID_STRIDE, RATE_TABLE_OFFSET,
    SPEED_CAP_GIVE_UP_AFTER_RESETS, SPEED_CAP_HELD_LOW_BEFORE_LIFT_MS, SPEED_CAP_LIFTED_VALUE,
    SPEED_CAP_RATE_CHANNEL, SPEED_CAP_RATE_ID, SPEED_CAP_THRESHOLD, STATE_ACTION_MOVE_RVA,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpeedCapLog {
    Lifted { from: f32 },
    GameReset { to: f32 },
    GaveUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpeedCapStep {
    pub write: bool,
    pub log: Option<SpeedCapLog>,
}

/// One tick's view of the swapped body's rate 5:2 and of what may hold it down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedCapReading {
    pub phase: u32,
    pub now_ms: u64,
    pub value: f32,
    pub kaine_swapped: bool,
    pub event_scene: bool,
    pub talk: bool,
    pub control_masked: bool,
    pub state_rva: Option<u32>,
}

/// Lifts rate 5:2 only when it has stayed low in the plain move state for a while (the house's
/// cap); the game's own short drops for dashes, attacks and stops are left alone.
#[derive(Debug, Default)]
pub struct SpeedCapLift {
    phase: u32,
    low_since_ms: Option<u64>,
    lifts: u32,
    reset_reported: bool,
    gave_up: bool,
}

impl SpeedCapLift {
    pub fn observe(&mut self, reading: SpeedCapReading) -> SpeedCapStep {
        if reading.phase != self.phase {
            *self = Self {
                phase: reading.phase,
                ..Self::default()
            };
        }
        if self.gave_up || !should_lift_speed_cap(&reading) {
            self.low_since_ms = None;
            return SpeedCapStep::default();
        }
        let low_since = *self.low_since_ms.get_or_insert(reading.now_ms);
        if reading.now_ms.saturating_sub(low_since) < SPEED_CAP_HELD_LOW_BEFORE_LIFT_MS {
            return SpeedCapStep::default();
        }
        self.low_since_ms = None;
        if self.lifts > SPEED_CAP_GIVE_UP_AFTER_RESETS {
            self.gave_up = true;
            return SpeedCapStep {
                write: false,
                log: Some(SpeedCapLog::GaveUp),
            };
        }
        let log = if self.lifts == 0 {
            Some(SpeedCapLog::Lifted {
                from: reading.value,
            })
        } else if self.reset_reported {
            None
        } else {
            self.reset_reported = true;
            Some(SpeedCapLog::GameReset { to: reading.value })
        };
        self.lifts += 1;
        SpeedCapStep { write: true, log }
    }

    pub fn lifts_this_phase(&self) -> u32 {
        self.lifts
    }
}

pub fn should_lift_speed_cap(reading: &SpeedCapReading) -> bool {
    reading.kaine_swapped
        && !reading.event_scene
        && !reading.talk
        && !reading.control_masked
        && reading.state_rva == Some(STATE_ACTION_MOVE_RVA)
        && reading.value < SPEED_CAP_THRESHOLD
}

pub fn speed_cap_address(actor: u64) -> u64 {
    actor
        + RATE_TABLE_OFFSET
        + SPEED_CAP_RATE_ID as u64 * RATE_TABLE_ID_STRIDE as u64
        + SPEED_CAP_RATE_CHANNEL as u64 * RATE_TABLE_CHANNEL_STRIDE as u64
}

pub fn read_speed_cap<M: GameMemory>(memory: &M, actor: u64) -> Option<f32> {
    memory.read_f32(speed_cap_address(actor)).ok()
}

pub fn write_lifted_speed_cap<M: GameMemory>(memory: &mut M, actor: u64) -> bool {
    memory
        .write_f32(speed_cap_address(actor), SPEED_CAP_LIFTED_VALUE)
        .is_ok()
}

pub fn format_speed_cap_log(log: SpeedCapLog, phase: u32) -> String {
    let held_s = SPEED_CAP_HELD_LOW_BEFORE_LIFT_MS as f32 / 1000.0;
    match log {
        SpeedCapLog::Lifted { from } => format!(
            "kaine walk: lifted the house speed cap {SPEED_CAP_RATE_ID}:{SPEED_CAP_RATE_CHANNEL} from {from:.2} to {SPEED_CAP_LIFTED_VALUE:.1} after {held_s:.1} s held low in the move state (phase {phase})"
        ),
        SpeedCapLog::GameReset { to } => format!(
            "kaine walk: the game held {SPEED_CAP_RATE_ID}:{SPEED_CAP_RATE_CHANNEL} at {to:.2} again for {held_s:.1} s, lifted again"
        ),
        SpeedCapLog::GaveUp => format!("kaine walk: gave up for phase {phase}"),
    }
}
