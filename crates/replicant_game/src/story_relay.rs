mod args;
mod call_gate;
mod gather_hold;
mod gather_verdict;
mod ledger;
mod lookup;
mod pull_arrival;
mod relay;

pub use args::{ArgsRefusal, args_block_len, args_hash, read_args_block, write_args_block};
pub use call_gate::{
    LongRelayWait, QueueRetry, RelayCallFlight, RelayCallGate, RelayCallNotice, RelayCallSubmitted,
    WaitingRelayCall,
};
pub use gather_hold::{
    GatherAdmission, GatherAloneReason, GatherInputs, GatherOutcome, GatherRelease, GatherStep,
    SceneGatherHold,
};
pub use gather_verdict::{
    GatherFacts, GatherStayReason, GatherVerdict, describe_partner_reply, gather_verdict,
};
pub use ledger::{CallKey, InjectedMatch, RefusedAction, StoryRelayLedger};
pub use lookup::{pack_hash_of_phase, relay_kind};
pub use pull_arrival::{PullArrival, PullArrivalInputs, PullArrivalStep};
pub use relay::{
    DropReason, FireAction, FireFacts, HeldTimeoutVerdict, RunAction, RunFacts, TriggerAction,
    held_timeout_verdict,
};

pub const QUEUE_SCRIPT_CALL_RVA: u32 = 0x41_1C20;
pub const MARK_QUEUED_CALL_READY_RVA: u32 = 0x41_3A70;
pub const QUEUED_CALL_ARGUMENTS_OFFSET: u64 = 0x48;
pub const QUEUED_CALL_READY_STATE: i32 = 2;
pub const QUEUED_CALL_FREE_STATE: i32 = 0;
pub const INJECTED_CALL_PRIORITY: i32 = 0;
pub const RELAY_CALL_POP_EXPECTED_MS: u64 = 5_000;
pub const MAX_WAITING_RELAY_CALLS: usize = 512;
pub const RELAY_CALL_STILL_QUEUED_LOGGED_AFTER_MS: u64 = 30_000;
pub const RELAY_CALL_LONG_WAIT_LOGGED_EVERY_MS: u64 = 5_000;
pub const RELAY_CALL_QUEUE_TRIES: u32 = 50;
pub const STORY_CALL_DEDUPE_WINDOW_MS: u64 = 2_000;
pub const INJECTED_CALL_MARK_LIFETIME_MS: u64 = 5_000;
pub const RAN_LOCALLY_AFTER_TIMEOUT_RECORD_MS: u64 = 10_000;
pub const OWN_STEP_SENT_REFUSES_ITS_TRIGGER_FOR_MS: u64 = 60_000;
pub const HELD_TRIGGER_ANSWER_TIMEOUT_MS: u64 = 1_500;
pub const LEDGER_MAX_ENTRIES_PER_LIST: usize = 64;
pub const ARGS_HEADER_BYTES: usize = 8;
pub const ARG_STRIDE_BYTES: usize = 0x90;
pub const ARG_VALUE_OFFSET: usize = 8;
pub const ARG_TEXT_OFFSET: usize = 0x10;
pub const ARG_TEXT_CAPACITY_BYTES: usize = 0x80;
pub const ARG_TAG_NUMBER: i32 = 1;
pub const ARG_TAG_TEXT: i32 = 2;
pub const ARG_TAG_POINTER: i32 = 3;
pub const ARG_TAG_INT: i32 = 4;
pub const NEVER_RELAYED_FUNCTIONS: [&str; 1] = ["SelectWarpPoint"];
pub const SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS: u64 = 2_000;
pub const PULL_ARRIVAL_SETTLE_MIN_MS: u64 = 300;
pub const PULL_ARRIVAL_SETTLE_LIMIT_MS: u64 = 1_200;
pub const GATHER_RELEASE_REMEMBERED_MS: u64 = 5_000;
pub const MAX_SCENE_CALLS_HELD_FOR_PARTNER: usize = 4;
