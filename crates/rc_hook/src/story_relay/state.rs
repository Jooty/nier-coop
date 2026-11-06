use std::sync::{Mutex, OnceLock};

use rc_game::story_relay::{CallKey, StoryRelayLedger};
use rc_proto::StoryCall;

use crate::build_gate::VerifiedBuild;

pub(super) static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
pub(super) static LEDGER: Mutex<StoryRelayLedger> = Mutex::new(StoryRelayLedger::new());
pub(super) static HELD_CALLS: Mutex<Vec<(CallKey, StoryCall)>> = Mutex::new(Vec::new());
