mod dedupe;
mod held_hp;
mod rule;

pub use dedupe::MessageDedupe;
pub use held_hp::held_hp_to_restore;
pub use rule::{
    GuestFacts, GuestVerdict, MirrorVerdict, SkipReason, guest_verdict, mirror_verdict,
};

pub const POST_MESSAGE_RVA: u32 = 0x40_F550;
pub const RAISED_MESSAGE_MASK_OFFSET: u64 = 0x9_F5E8;
pub const RAISED_MESSAGE_MASK_BYTES: usize = 128;
pub const FIRST_MIRRORED_MESSAGE: u32 = 241;
pub const LAST_MIRRORED_MESSAGE: u32 = 299;
pub const NEVER_MIRRORED_MESSAGES: [u32; 7] = [210, 213, 215, 470, 471, 472, 473];
pub const MESSAGE_DEDUPE_WINDOW_MS: u64 = 100;
pub const MESSAGE_DEDUPE_MAX_ENTRIES: usize = 32;
pub const MAX_LOGGED_UNMIRRORED_NUMBERS: usize = 64;
