use std::sync::{Mutex, OnceLock};

use rc_game::story_relay::StoryRelayLedger;

use crate::build_gate::VerifiedBuild;

pub(super) static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
pub(super) static LEDGER: Mutex<StoryRelayLedger> = Mutex::new(StoryRelayLedger::new());
