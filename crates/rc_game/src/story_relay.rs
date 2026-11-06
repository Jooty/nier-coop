mod args;
mod ledger;
mod lookup;
mod relay;

pub use args::{ArgsRefusal, args_block_len, args_hash, read_args_block, write_args_block};
pub use ledger::{CallKey, StoryRelayLedger};
pub use lookup::{pack_hash_of_phase, relay_kind};
pub use relay::{DropReason, FireAction, FireFacts, RunAction, RunFacts, TriggerAction};

pub const QUEUE_SCRIPT_CALL_RVA: u32 = 0x41_1C20;
pub const MARK_QUEUED_CALL_READY_RVA: u32 = 0x41_3A70;
pub const QUEUED_CALL_ARGUMENTS_OFFSET: u64 = 0x48;
pub const INJECTED_CALL_PRIORITY: i32 = 0;
pub const STORY_CALL_DEDUPE_WINDOW_MS: u64 = 2_000;
pub const INJECTED_CALL_MARK_LIFETIME_MS: u64 = 5_000;
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
