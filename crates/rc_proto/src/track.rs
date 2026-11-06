mod state_track;
mod track_sample;

pub use state_track::StateTrack;
pub use track_sample::TrackSample;

pub const MAX_TRACK_SAMPLES: usize = 64;
pub const TRACK_HISTORY_MS: u64 = 500;
pub const TRACK_RESTART_GAP_MS: u64 = 1000;
pub const TRACK_RESTART_TICK_JUMP: i32 = 1000;
