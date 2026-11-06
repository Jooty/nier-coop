mod actor_liveness;
mod affine;
mod attach_point;
mod layer_motion;
mod motion_follow;
mod pose;
mod weapon_look;

pub use actor_liveness::actor_is_live;
pub use affine::Affine;
pub use attach_point::{AttachPoint, read_attach_point};
pub use layer_motion::{
    LayerMotion, PuppetLayer, motion_name, read_layer_motion, read_puppet_layer, seek_layer_frame,
};
pub use motion_follow::{MotionStep, follow_step, frame_to_seek, wrapped_frame};
pub use pose::PuppetPose;
pub use weapon_look::{read_weapon_grip, read_weapon_look};

pub const MOTION_SEEK_TOLERANCE_FRAMES: f32 = 0.5;
pub const IDLE_MOTION_NAME: &str = "NIER013_idle_lp";
pub const MOTION_BLEND_SECONDS: f32 = 0.1;
