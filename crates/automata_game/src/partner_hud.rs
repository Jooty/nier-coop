mod tag_camera;
mod tag_rules;

pub use tag_camera::{Projected, TagCamera, project_tag};
pub use tag_rules::{TagInputs, shown_name, tag_fade_alpha, tag_hidden_reason, tag_label};

pub const CAMERA_TRANSFORM_POINTER_OFFSET: u32 = 0x60;
pub const CAMERA_VIEW_MATRIX_OFFSET: u64 = 0x0;
pub const CAMERA_PROJECTION_MATRIX_OFFSET: u64 = 0x40;
pub const MATRIX_FLOATS: usize = 16;
pub const NEAREST_TAG_DEPTH_UNITS: f32 = 0.05;
pub const TAG_HEAD_OFFSET_UNITS: f32 = 1.9;
pub const TAG_EDGE_MARGIN_FRACTION: f32 = 0.08;
pub const TAG_PARTNER_STATE_STALE_MS: u64 = 3000;
pub const TAG_FADE_START_DISTANCE_UNITS: f32 = 60.0;
pub const TAG_FADE_END_DISTANCE_UNITS: f32 = 200.0;
pub const TAG_FADE_MINIMUM_ALPHA: f32 = 0.35;
pub const SHOWN_NAME_MAX_CHARS: usize = 24;
pub const NAME_OF_AN_UNNAMED_PARTNER: &str = "Partner";
pub const HUD_REFERENCE_SCREEN_HEIGHT: f32 = 1080.0;
pub const CHARACTERS_THE_GAME_TEXT_MAY_READ_AS_MARKUP: [char; 4] = ['<', '>', '\\', '%'];
