use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use rc_game::boss_messages::RaisedSeqs;

use crate::build_gate::VerifiedBuild;

pub(super) static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
pub(super) static ORIGINAL_POST_MESSAGE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
pub(super) static LAST_SENT_SEQ: AtomicU32 = AtomicU32::new(0);
pub(super) static RAISED_SEQS: Mutex<RaisedSeqs> = Mutex::new(RaisedSeqs::new());
pub(super) static LOGGED_UNMIRRORED: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static MESSAGES_SENT: AtomicU32 = AtomicU32::new(0);
static MESSAGES_RAISED: AtomicU32 = AtomicU32::new(0);
static MESSAGES_SKIPPED: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BossMessageCounts {
    pub sent: u32,
    pub raised: u32,
    pub skipped: u32,
}

pub(super) enum CountedAs {
    Sent,
    Raised,
    Skipped,
}

pub(super) fn count(kind: CountedAs) {
    let counter = match kind {
        CountedAs::Sent => &MESSAGES_SENT,
        CountedAs::Raised => &MESSAGES_RAISED,
        CountedAs::Skipped => &MESSAGES_SKIPPED,
    };
    counter.fetch_add(1, Ordering::Relaxed);
}

pub fn boss_message_counts() -> BossMessageCounts {
    BossMessageCounts {
        sent: MESSAGES_SENT.load(Ordering::Relaxed),
        raised: MESSAGES_RAISED.load(Ordering::Relaxed),
        skipped: MESSAGES_SKIPPED.load(Ordering::Relaxed),
    }
}
