mod apply;
mod force_kill;
mod status;
mod tick;

pub use status::{downed_status, request_go_down, request_revive_partner};
pub use tick::tick_downed;

use std::time::Duration;

const MAX_STEP_SECONDS: f32 = 0.25;
const PARTNER_STATE_MAX_AGE: Duration = Duration::from_secs(1);
