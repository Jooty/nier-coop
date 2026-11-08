mod held_hp;
mod raised_seqs;
mod rule;

pub use held_hp::{LifeCallbackEntry, held_hp_to_restore, life_callback_targets_group};
pub use raised_seqs::RaisedSeqs;
pub use rule::{
    GuestFacts, GuestVerdict, MirrorVerdict, SkipReason, guest_verdict, mirror_verdict,
};

pub const POST_MESSAGE_RVA: u32 = 0x40_F550;
pub const RAISED_MESSAGE_MASK_OFFSET: u64 = 0x9_F5E8;
pub const RAISED_MESSAGE_MASK_BYTES: usize = 128;
pub const FIRST_MIRRORED_MESSAGE: u32 = 241;
pub const LAST_MIRRORED_MESSAGE: u32 = 299;
pub const NEVER_MIRRORED_MESSAGES: [u32; 7] = [210, 213, 215, 470, 471, 472, 473];
pub const RAISED_SEQ_MEMORY_ENTRIES: usize = 128;
pub const LIFE_CALLBACK_KIND: i32 = 0xE;
pub const MAX_LOGGED_UNMIRRORED_NUMBERS: usize = 64;
