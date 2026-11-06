mod actor_liveness;
mod layer_motion;
mod pose;

pub use actor_liveness::actor_is_live;
pub use layer_motion::{LayerMotion, motion_name, read_layer_motion};
pub use pose::PuppetPose;

pub const MIRROR_FRAME_DRIFT_LIMIT: f32 = 4.0;
pub const MIRROR_BLEND_FRAMES: f32 = 6.0;
