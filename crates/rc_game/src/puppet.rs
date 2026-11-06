mod actor_liveness;
mod delay_line;
mod layer_motion;
mod pose;

pub use actor_liveness::actor_is_live;
pub use delay_line::DelayLine;
pub use layer_motion::{LayerMotion, motion_name, read_layer_motion};
pub use pose::{PuppetPose, read_pose};

pub const MAX_MIRROR_DELAY_MS: u16 = 500;
pub const MIRROR_SIDE_OFFSET_UNITS: f32 = 2.0;
pub const MIRROR_FRAME_DRIFT_LIMIT: f32 = 4.0;
pub const MIRROR_BLEND_FRAMES: f32 = 6.0;
