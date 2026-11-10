mod channel;
mod guest_sync;
mod sender;

pub use channel::{MusicChannel, read_music_channel};
pub use guest_sync::{GuestSync, LocalMusic, MusicStep};
pub use sender::MusicSender;

pub const MUSIC_REFRESH_INTERVAL_MS: u64 = 1000;
pub const MUSIC_MIN_CHANGE_INTERVAL_MS: u64 = 100;
pub const MUSIC_TARGET_STALE_MS: u64 = 4000;
pub const MUSIC_DRIFT_RESEEK_MS: u32 = 750;
pub const MUSIC_RESEEK_MIN_INTERVAL_MS: u64 = 5000;
pub const MUSIC_PLAY_RETRY_INTERVAL_MS: u64 = 500;
pub const MUSIC_SEEK_WAIT_LIMIT_MS: u64 = 3000;
pub const MUSIC_VOLUME_EPSILON: f32 = 0.001;
