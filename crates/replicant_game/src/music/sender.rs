use replicant_proto::MusicState;

use crate::music::{MUSIC_MIN_CHANGE_INTERVAL_MS, MUSIC_REFRESH_INTERVAL_MS, MUSIC_VOLUME_EPSILON};

#[derive(Debug, Clone, Default)]
pub struct MusicSender {
    last: Option<(MusicState, u64)>,
}

impl MusicSender {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn send_next_now(&mut self) {
        self.last = None;
    }

    pub fn should_send(&mut self, now_ms: u64, state: &MusicState) -> bool {
        let Some((previous, sent_at)) = self.last else {
            self.last = Some((*state, now_ms));
            return true;
        };
        let age = now_ms.saturating_sub(sent_at);
        let changed = !same_mix(&previous, state);
        let due = if changed {
            age >= MUSIC_MIN_CHANGE_INTERVAL_MS
        } else {
            age >= MUSIC_REFRESH_INTERVAL_MS
        };
        if due {
            self.last = Some((*state, now_ms));
        }
        due
    }
}

fn same_mix(a: &MusicState, b: &MusicState) -> bool {
    let close = |x: f32, y: f32| (x - y).abs() <= MUSIC_VOLUME_EPSILON;
    a.track == b.track
        && a.stopping == b.stopping
        && close(a.track_volume, b.track_volume)
        && close(a.master_volume, b.master_volume)
        && a.layer_volumes
            .iter()
            .zip(&b.layer_volumes)
            .all(|(x, y)| close(*x, *y))
}
