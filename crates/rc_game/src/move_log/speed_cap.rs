use crate::memory::GameMemory;
use crate::move_log::{
    RATE_TABLE_CHANNEL_STRIDE, RATE_TABLE_ID_STRIDE, RATE_TABLE_OFFSET,
    SPEED_CAP_GIVE_UP_AFTER_RESETS, SPEED_CAP_LIFTED_VALUE, SPEED_CAP_RATE_CHANNEL,
    SPEED_CAP_RATE_ID, SPEED_CAP_THRESHOLD,
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

#[derive(Debug, Default)]
pub struct SpeedCapLift {
    phase: u32,
    lifted_once: bool,
    reset_reported: bool,
    resets: u32,
    gave_up: bool,
}

impl SpeedCapLift {
    pub fn observe(
        &mut self,
        phase: u32,
        value: f32,
        kaine_swapped: bool,
        event_scene: bool,
        talk: bool,
    ) -> SpeedCapStep {
        if phase != self.phase {
            *self = Self {
                phase,
                ..Self::default()
            };
        }
        if self.gave_up || !should_lift_speed_cap(value, kaine_swapped, event_scene, talk) {
            return SpeedCapStep::default();
        }
        let mut log = None;
        if self.lifted_once {
            self.resets += 1;
            if self.resets > SPEED_CAP_GIVE_UP_AFTER_RESETS {
                self.gave_up = true;
                return SpeedCapStep {
                    write: false,
                    log: Some(SpeedCapLog::GaveUp),
                };
            }
            if !self.reset_reported {
                self.reset_reported = true;
                log = Some(SpeedCapLog::GameReset { to: value });
            }
        } else {
            self.lifted_once = true;
            log = Some(SpeedCapLog::Lifted { from: value });
        }
        SpeedCapStep { write: true, log }
    }
}

pub fn should_lift_speed_cap(value: f32, kaine_swapped: bool, event_scene: bool, talk: bool) -> bool {
    kaine_swapped && !event_scene && !talk && value < SPEED_CAP_THRESHOLD
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
    match log {
        SpeedCapLog::Lifted { from } => format!(
            "kaine walk: lifted the house speed cap {SPEED_CAP_RATE_ID}:{SPEED_CAP_RATE_CHANNEL} from {from:.2} to {SPEED_CAP_LIFTED_VALUE:.1} (phase {phase})"
        ),
        SpeedCapLog::GameReset { to } => format!(
            "kaine walk: the game reset {SPEED_CAP_RATE_ID}:{SPEED_CAP_RATE_CHANNEL} to {to:.2}"
        ),
        SpeedCapLog::GaveUp => format!("kaine walk: gave up for phase {phase}"),
    }
}
