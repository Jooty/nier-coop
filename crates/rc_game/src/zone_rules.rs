mod exit_names;
mod prompt;
mod radius;
mod verdict;

pub use exit_names::is_zone_exit_callback;
pub use prompt::{VetoNotice, VetoRecord};
pub use radius::ZoneRadius;
pub use verdict::{
    Companion, Placement, ZoneChangeAsk, ZoneChangeSource, ZoneVerdict, judge_zone_change,
};

pub const DEFAULT_ZONE_RADIUS_UNITS: f32 = 15.0;
pub const MIN_ZONE_RADIUS_UNITS: f32 = 3.0;
pub const MAX_ZONE_RADIUS_UNITS: f32 = 200.0;
pub const VETO_PROMPT_VISIBLE_MS: u64 = 3000;
pub const VETO_LOG_INTERVAL_MS: u64 = 1000;
pub const GUEST_VETO_TEXT: &str = "Only the host can change zones";
pub const ZONE_EXIT_CALLBACK_NAME_PARTS: [&str; 4] = ["jumpmap", "jump_map", "mapjump", "map_jump"];
