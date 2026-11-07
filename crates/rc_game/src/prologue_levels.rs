mod catch_up;

pub use catch_up::{LevelCatchUp, LevelStep, LocalLevel, PartnerLevel};

pub const PROLOGUE_PHASES: [&str; 1] = ["A_TOKYO_FIELD_01"];
pub const LEVEL_RESEND_INTERVAL_MS: u64 = 5_000;
pub const CATCH_UP_SETTLE_MS: u64 = 1_500;
