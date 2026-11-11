use replicant_proto::SkyState;

use crate::sky::{APPLY_RETRY_MS, SkyReading};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SkyApply {
    pub weather: Option<(u8, u8)>,
    pub env: Option<(i32, f32)>,
    pub wind: Option<(i32, f32)>,
}

#[derive(Debug, Default)]
pub struct SkyHold {
    target: Option<SkyState>,
    fresh: bool,
    applied_before_resources_ready: bool,
    last_attempt_ms: Option<u64>,
}

impl SkyApply {
    pub fn is_empty(&self) -> bool {
        self.weather.is_none() && self.env.is_none() && self.wind.is_none()
    }
}

impl SkyHold {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn receive(&mut self, state: SkyState) {
        self.target = Some(state);
        self.fresh = true;
        self.last_attempt_ms = None;
    }

    pub fn step(&mut self, now_ms: u64, guest_phase: u16, reading: &SkyReading) -> SkyApply {
        let Some(target) = self.target else {
            return SkyApply::default();
        };
        if target.phase != guest_phase || !reading.is_settled() {
            return SkyApply::default();
        }
        let blend = if self.fresh {
            target.env_blend_seconds
        } else {
            0.0
        };
        let env_mismatch = reading.env_area != target.env_area;
        let resources_arrived = self.applied_before_resources_ready && reading.sky_resources_ready;
        let apply = SkyApply {
            weather: (reading.weather != target.weather
                || reading.weather_offset != target.weather_offset)
                .then_some((target.weather, target.weather_offset)),
            env: (env_mismatch || resources_arrived)
                .then_some((target.env_area, if env_mismatch { blend } else { 0.0 })),
            wind: (target.wind_area >= 0 && reading.wind_area != target.wind_area)
                .then_some((target.wind_area, blend)),
        };
        if apply.is_empty() {
            self.fresh = false;
            self.last_attempt_ms = None;
            return apply;
        }
        if !self.fresh
            && self
                .last_attempt_ms
                .is_some_and(|last| now_ms.saturating_sub(last) < APPLY_RETRY_MS)
        {
            return SkyApply::default();
        }
        self.last_attempt_ms = Some(now_ms);
        self.fresh = false;
        if apply.env.is_some() {
            self.applied_before_resources_ready = env_mismatch && !reading.sky_resources_ready;
        }
        apply
    }
}
