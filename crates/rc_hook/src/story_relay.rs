mod dummy_hits;
mod fire;
pub(crate) mod inject;
mod pack_log;
mod settings;
mod state;
mod tick;

pub use fire::{ms_since_an_injected_run_started, suppress_fired_call};
pub use settings::{set_shared_story_events, shared_story_events};
pub use tick::{install, tick};
