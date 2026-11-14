use crate::move_log::{
    SPEED_CAP_RATE_CHANNEL, SPEED_CAP_RATE_ID, SPEED_CAP_REPORT_SECONDS_AFTER_LOAD,
    state_name_for_rva,
};

/// The second-by-second watch of the swapped body's speed after each phase load.
#[derive(Debug, Default)]
pub struct SpeedCapWatch {
    phase: u32,
    first_tick_ms: u64,
    last_second: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpeedCapSecond {
    pub phase: u32,
    pub seconds_after_load: u64,
    pub cap: Option<f32>,
    pub lifts_this_phase: u32,
    pub step_per_second: Option<f32>,
    pub ground_speed: Option<f32>,
    pub state_rva: Option<u32>,
}

impl SpeedCapWatch {
    pub fn due(&mut self, phase: u32, now_ms: u64) -> Option<u64> {
        if phase != self.phase || self.first_tick_ms == 0 {
            *self = Self {
                phase,
                first_tick_ms: now_ms.max(1),
                last_second: None,
            };
        }
        let second = now_ms.saturating_sub(self.first_tick_ms) / 1000;
        if second >= SPEED_CAP_REPORT_SECONDS_AFTER_LOAD
            || self.last_second.is_some_and(|last| last >= second)
        {
            return None;
        }
        self.last_second = Some(second);
        Some(second)
    }
}

pub fn format_speed_cap_second(second: &SpeedCapSecond) -> String {
    let number = |value: Option<f32>| value.map_or_else(|| "?".to_owned(), |v| format!("{v:.2}"));
    let state = second.state_rva.map_or("none", state_name_for_rva);
    format!(
        "kaine walk: phase {} +{}s: {SPEED_CAP_RATE_ID}:{SPEED_CAP_RATE_CHANNEL} {} ({} lifts this phase), step {}/s at ground speed {}, state {state}",
        second.phase,
        second.seconds_after_load,
        number(second.cap),
        second.lifts_this_phase,
        number(second.step_per_second),
        number(second.ground_speed),
    )
}
