use rc_proto::{MUSIC_LAYER_COUNT, MusicState, NO_MUSIC_TRACK};

use crate::music::{
    MUSIC_DRIFT_RESEEK_MS, MUSIC_PLAY_RETRY_INTERVAL_MS, MUSIC_RESEEK_MIN_INTERVAL_MS,
    MUSIC_SEEK_WAIT_LIMIT_MS, MUSIC_TARGET_STALE_MS, MUSIC_VOLUME_EPSILON, MusicChannel,
};
use crate::offsets::BGM_CHANNEL_STATE_EVENT_POSTED;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalMusic {
    pub channel: MusicChannel,
    pub position_ms: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MusicStep {
    Play { track: i32, volume: f32 },
    Stop { fade_seconds: f32 },
    Seek { position_ms: u32 },
    SeekGaveUp { event_track: i32, state: i32 },
    Master { volume: f32 },
    Layer { layer: u8, volume: f32 },
}

#[derive(Clone, Copy, Debug)]
struct Target {
    state: MusicState,
    received_at_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct GuestSync {
    target: Option<Target>,
    seek_wanted_since_ms: Option<u64>,
    last_play_ms: Option<u64>,
    last_stop_ms: Option<u64>,
    last_seek_ms: Option<u64>,
}

impl GuestSync {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn receive(&mut self, state: MusicState, now_ms: u64) {
        let track_changed = self
            .target
            .is_none_or(|target| target.state.track != state.track);
        if track_changed {
            self.seek_wanted_since_ms = None;
        }
        self.target = Some(Target {
            state,
            received_at_ms: now_ms,
        });
    }

    pub fn is_fresh(&self, now_ms: u64) -> bool {
        self.target.is_some_and(|target| {
            now_ms.saturating_sub(target.received_at_ms) <= MUSIC_TARGET_STALE_MS
        })
    }

    pub fn forget(&mut self) {
        *self = Self::default();
    }

    pub fn expected_position_ms(&self, now_ms: u64, latency_ms: u64) -> Option<u32> {
        let target = self.target?;
        let elapsed = now_ms.saturating_sub(target.received_at_ms) + latency_ms;
        let moved = if target.state.track == NO_MUSIC_TRACK || target.state.stopping {
            0
        } else {
            elapsed
        };
        let total = u64::from(target.state.position_ms) + moved;
        Some(u32::try_from(total).unwrap_or(u32::MAX))
    }

    pub fn plan(&mut self, local: &LocalMusic, now_ms: u64, latency_ms: u64) -> Vec<MusicStep> {
        let Some(target) = self.target else {
            return Vec::new();
        };
        let wanted = target.state;
        let mut steps = Vec::new();
        if wanted.track == NO_MUSIC_TRACK {
            self.plan_stop(local, now_ms, 0.0, &mut steps);
        } else if wanted.stopping {
            self.plan_stop(local, now_ms, wanted.stop_fade_seconds, &mut steps);
        } else {
            self.plan_track(local, now_ms, latency_ms, &mut steps);
        }
        if wanted.track != NO_MUSIC_TRACK && !wanted.stopping {
            plan_volumes(&wanted, &local.channel, &mut steps);
        }
        steps
    }

    fn plan_stop(
        &mut self,
        local: &LocalMusic,
        now_ms: u64,
        fade_seconds: f32,
        steps: &mut Vec<MusicStep>,
    ) {
        self.seek_wanted_since_ms = None;
        let running = local.channel.has_track() && !local.channel.stopping;
        if running && elapsed_since(self.last_stop_ms, now_ms) >= MUSIC_PLAY_RETRY_INTERVAL_MS {
            self.last_stop_ms = Some(now_ms);
            steps.push(MusicStep::Stop { fade_seconds });
        }
    }

    fn plan_track(
        &mut self,
        local: &LocalMusic,
        now_ms: u64,
        latency_ms: u64,
        steps: &mut Vec<MusicStep>,
    ) {
        let Some(target) = self.target else {
            return;
        };
        let wanted = target.state;
        let channel = &local.channel;
        let on_track = channel.track == wanted.track && !channel.stopping;
        if !on_track {
            if elapsed_since(self.last_play_ms, now_ms) >= MUSIC_PLAY_RETRY_INTERVAL_MS {
                self.last_play_ms = Some(now_ms);
                self.seek_wanted_since_ms = Some(now_ms);
                steps.push(MusicStep::Play {
                    track: wanted.track,
                    volume: wanted.track_volume,
                });
            }
            return;
        }
        let event_ready = channel.state == BGM_CHANNEL_STATE_EVENT_POSTED
            && channel.playing_id != 0
            && channel.game_object != 0
            && channel.event_track == wanted.track;
        let Some(expected) = self.expected_position_ms(now_ms, latency_ms) else {
            return;
        };
        if !event_ready {
            if let Some(since) = self.seek_wanted_since_ms
                && now_ms.saturating_sub(since) >= MUSIC_SEEK_WAIT_LIMIT_MS
            {
                self.seek_wanted_since_ms = None;
                steps.push(MusicStep::SeekGaveUp {
                    event_track: channel.event_track,
                    state: channel.state,
                });
            }
            return;
        }
        let drifted = local
            .position_ms
            .is_some_and(|position| position.abs_diff(expected) > MUSIC_DRIFT_RESEEK_MS);
        let drift_allowed =
            elapsed_since(self.last_seek_ms, now_ms) >= MUSIC_RESEEK_MIN_INTERVAL_MS;
        if self.seek_wanted_since_ms.is_some() || (drifted && drift_allowed) {
            self.seek_wanted_since_ms = None;
            self.last_seek_ms = Some(now_ms);
            steps.push(MusicStep::Seek {
                position_ms: expected,
            });
        }
    }
}

fn plan_volumes(wanted: &MusicState, channel: &MusicChannel, steps: &mut Vec<MusicStep>) {
    let differs = |a: f32, b: f32| (a - b).abs() > MUSIC_VOLUME_EPSILON;
    if differs(wanted.master_volume, channel.master_volume) {
        steps.push(MusicStep::Master {
            volume: wanted.master_volume,
        });
    }
    for layer in 0..MUSIC_LAYER_COUNT {
        if differs(wanted.layer_volumes[layer], channel.layer_volumes[layer]) {
            steps.push(MusicStep::Layer {
                layer: layer as u8,
                volume: wanted.layer_volumes[layer],
            });
        }
    }
}

fn elapsed_since(at: Option<u64>, now_ms: u64) -> u64 {
    at.map_or(u64::MAX, |at| now_ms.saturating_sub(at))
}
