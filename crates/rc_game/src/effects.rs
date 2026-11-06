mod anchor_offset;
mod capture;
mod falloff;
mod instance;
mod live_effects;
mod rate_window;
mod sent_effects;

pub use anchor_offset::{match_anchor_offset, read_anchor_offset};
pub use capture::{CapturedEffect, EffectCall, build_effect_call, read_effect, read_sound};
pub use falloff::sound_gain;
pub use instance::{effect_instance_address, effect_is_finished, finish_effect};
pub use live_effects::LiveEffects;
pub use rate_window::RateWindow;
pub use sent_effects::SentEffects;

pub const SOUND_FALLOFF_NEAR_UNITS: f32 = 6.0;
pub const SOUND_FALLOFF_FAR_UNITS: f32 = 45.0;
pub const MAX_LIVE_PUPPET_EFFECTS: usize = 64;
pub const PUPPET_EFFECT_MAX_LIFETIME_MS: u64 = 8000;
pub const MAX_EFFECT_EVENTS_PER_SECOND: u32 = 40;
pub const MAX_PUPPET_EFFECTS_PER_TICK: usize = 8;
pub const EFFECT_LOG_LINES_PER_WINDOW: u32 = 12;
pub const EFFECT_LOG_WINDOW_MS: u64 = 5000;
