use rc_proto::SkyState;

use crate::sky::{SkyReading, ZONE_SETTLE_MS};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkySend {
    pub state: SkyState,
    pub reason: &'static str,
}

#[derive(Debug, Default)]
pub struct SkyBroadcast {
    last_sent: Option<SkyState>,
    pending_reason: Option<&'static str>,
    guest_seen: bool,
    guest_phase: Option<u16>,
    host_phase: Option<u16>,
    quiet_until_ms: u64,
}

impl SkyBroadcast {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn next(
        &mut self,
        now_ms: u64,
        host_phase: u16,
        reading: &SkyReading,
        guest_phase: Option<u16>,
    ) -> Option<SkySend> {
        let Some(guest_phase) = guest_phase else {
            self.guest_seen = false;
            self.guest_phase = None;
            return None;
        };
        if !self.guest_seen {
            self.guest_seen = true;
            self.pending_reason = Some("guest joined");
        } else if self.guest_phase != Some(guest_phase) {
            self.pending_reason = Some("guest zone loaded");
            self.quiet_until_ms = now_ms + ZONE_SETTLE_MS;
        }
        if self.host_phase.is_some_and(|phase| phase != host_phase) {
            self.pending_reason = Some("host zone loaded");
            self.quiet_until_ms = now_ms + ZONE_SETTLE_MS;
        }
        self.guest_phase = Some(guest_phase);
        self.host_phase = Some(host_phase);
        if now_ms < self.quiet_until_ms || guest_phase != host_phase || !reading.is_settled() {
            return None;
        }
        let state = SkyState {
            phase: host_phase,
            env_area: reading.env_area,
            env_blend_seconds: reading.env_blend_seconds,
            wind_area: reading.wind_area,
            weather: reading.weather,
            weather_offset: reading.weather_offset,
        };
        let changed = self.last_sent.is_none_or(|sent| !same_sky(&sent, &state));
        let reason = match (self.pending_reason.take(), changed) {
            (Some(reason), _) => reason,
            (None, true) => "host sky changed",
            (None, false) => return None,
        };
        self.last_sent = Some(state);
        let state = if reason == "host sky changed" {
            state
        } else {
            SkyState {
                env_blend_seconds: 0.0,
                ..state
            }
        };
        Some(SkySend { state, reason })
    }
}

fn same_sky(left: &SkyState, right: &SkyState) -> bool {
    left.phase == right.phase
        && left.env_area == right.env_area
        && left.wind_area == right.wind_area
        && left.weather == right.weather
        && left.weather_offset == right.weather_offset
}
