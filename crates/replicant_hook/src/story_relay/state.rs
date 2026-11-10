use std::sync::{Mutex, OnceLock};

use replicant_game::story_relay::{CallKey, SceneGatherHold, StoryRelayLedger};
use replicant_proto::StoryCall;

use crate::build_gate::VerifiedBuild;

pub(super) static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
pub(super) static LEDGER: Mutex<StoryRelayLedger> = Mutex::new(StoryRelayLedger::new());
pub(super) static HELD_CALLS: Mutex<Vec<(CallKey, StoryCall)>> = Mutex::new(Vec::new());
pub(super) static LAST_RUN_STARTED_BY_INJECTION_AT_MS: Mutex<Option<u64>> = Mutex::new(None);
pub(super) static GATHER_HOLD: Mutex<SceneGatherHold> = Mutex::new(SceneGatherHold::new());
