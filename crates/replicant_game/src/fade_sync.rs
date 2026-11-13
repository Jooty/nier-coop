mod guest;
mod host;
mod scene_cover;

pub use guest::{FadeEdge, FadeWatch, GuestFadeState};
pub use host::HostFadeSync;
pub use scene_cover::{CoverLift, SceneCover, SceneCovers};

pub const FADE_LAYER_RVA: u32 = 0x4B1_C860;
pub const EVENT_FADE_LAYER_RVA: u32 = 0x4B1_C6E0;
pub const FADE_OUT_FUNCTION_RVA: u32 = 0x42_B280;
pub const FADE_IN_FUNCTION_RVA: u32 = 0x42_B140;
pub const FADE_ACTIVE_OFFSET: u64 = 0x144;
pub const FADE_ALPHA_OFFSET: u64 = 0x148;
pub const FADE_OPAQUE_ALPHA: f32 = 255.0;
pub const ZONE_CHANGE_FADE_WINDOW_MS: u64 = 20_000;
pub const GUEST_FADE_BACKSTOP_MS: u64 = 15_000;
pub const NO_CUT_REVEAL_FADE_KIND: i32 = 1;
