mod minimap_dot;
mod pause_word_watch;
mod tag_camera;
mod tag_rules;

pub use minimap_dot::{
    GameMinimap, MinimapDot, MinimapTuning, minimap_dot, minimap_hidden_reason,
    minimap_update_start, ui_point_on_display,
};
pub use pause_word_watch::{PauseWordFacts, PauseWordWatch, pause_word_line};
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
pub const PAUSE_WORD_LINES_LOGGED_PER_MINUTE: u32 = 60;
pub const SHOWN_NAME_MAX_CHARS: usize = 24;
pub const NAME_OF_AN_UNNAMED_PARTNER: &str = "Partner";
pub const HUD_REFERENCE_SCREEN_HEIGHT: f32 = 1080.0;
pub const HUD_REFERENCE_SCREEN_WIDTH: f32 = 1920.0;
pub const MINIMAP_DOT_EDGE_INSET_FRACTION: f32 = 0.08;
pub const MINIMAP_NUDGE_RANGE: (f32, f32) = (-200.0, 200.0);
pub const MINIMAP_DOT_SIZE_RANGE: (f32, f32) = (2.0, 30.0);
pub const CAMERA_VIEWPORT_OFFSET: u64 = 0x258;
pub const MINIMAP_OBJECT_OPEN_STATE_OFFSET: u64 = 0x88;
pub const MINIMAP_OBJECT_ZOOM_OFFSET: u64 = 0xA0;
pub const MINIMAP_OBJECT_KIND_OFFSET: u64 = 0xB4;
pub const MINIMAP_OBJECT_ROTATION_OFFSET: u64 = 0x100;
pub const MINIMAP_OPENING_STATE: i32 = 1;
pub const MINIMAP_OPEN_STATE: i32 = 2;
pub const MINIMAP_SIDE_VIEW_KIND: i32 = 2;
pub const MINIMAP_READING_STALE_MS: u64 = 250;
pub const MINIMAP_UPDATE_FUNCTION_MAIN_START: [u8; 22] = [
    0x48, 0x8B, 0xC4, 0x55, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D, 0xA8, 0xD8,
    0xFE, 0xFF, 0xFF, 0x48, 0x81, 0xEC,
];
pub const MINIMAP_UPDATE_FUNCTION_COMPAT_START: [u8; 24] = [
    0x48, 0x8B, 0xC4, 0x55, 0x56, 0x57, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x8D,
    0xA8, 0xD8, 0xFE, 0xFF, 0xFF, 0x48, 0x81, 0xEC,
];
pub const CHARACTERS_THE_GAME_TEXT_MAY_READ_AS_MARKUP: [char; 4] = ['<', '>', '\\', '%'];
