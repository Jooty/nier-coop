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
pub const PLAYER_CODE_POSTED_MESSAGES: [u32; 12] =
    [210, 211, 212, 213, 215, 237, 239, 470, 471, 473, 480, 481];
pub const MIXED_POSTER_MESSAGES_OUTSIDE_MIRRORED_RANGE: [u32; 18] = [
    227, 234, 238, 240, 315, 316, 317, 340, 341, 342, 343, 351, 361, 420, 422, 483, 484, 490,
];
pub const STATE_STEP_ONLY_MESSAGES_OUTSIDE_MIRRORED_RANGE: [u32; 21] = [
    216, 219, 220, 221, 222, 223, 224, 225, 226, 228, 229, 231, 305, 306, 326, 327, 328, 329, 332,
    380, 495,
];
pub const RAISED_SEQ_MEMORY_ENTRIES: usize = 128;
pub const LIFE_CALLBACK_KIND: i32 = 0xE;
pub const MAX_LOGGED_UNMIRRORED_NUMBERS: usize = 64;
