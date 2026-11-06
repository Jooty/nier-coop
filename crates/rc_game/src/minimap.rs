mod dot;
mod tuning;

pub use dot::{MinimapDot, minimap_dot};
pub use tuning::MinimapTuning;

pub const MINIMAP_DOT_EDGE_INSET_FRACTION: f32 = 0.08;
pub const MINIMAP_REFERENCE_HEIGHT_PIXELS: f32 = 1080.0;
pub const MINIMAP_CENTER_X_RANGE: (f32, f32) = (0.0, 1920.0);
pub const MINIMAP_CENTER_Y_RANGE: (f32, f32) = (0.0, 1080.0);
pub const MINIMAP_RADIUS_RANGE: (f32, f32) = (20.0, 300.0);
pub const MINIMAP_WORLD_RANGE_RANGE: (f32, f32) = (10.0, 500.0);
pub const MINIMAP_DOT_SIZE_RANGE: (f32, f32) = (2.0, 30.0);
