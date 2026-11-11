use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

use replicant_game::downed::{GAME_OVER_MESSAGE, HostGameOverGate};

use crate::build_gate::VerifiedBuild;
use crate::net::net_clock_ms;

pub(super) static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
pub(super) static ORIGINAL_BACKUP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
pub(super) static ORIGINAL_RESTORE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
pub(super) static GAME_OVER_GATE: Mutex<HostGameOverGate> = Mutex::new(HostGameOverGate::new());
static REWINDS: AtomicU32 = AtomicU32::new(0);
static REWOUND_NOTICE: AtomicBool = AtomicBool::new(false);

pub fn arm_game_over() {
    GAME_OVER_GATE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .arm(net_clock_ms());
}

pub fn note_posted_message(message: u32) {
    if message == GAME_OVER_MESSAGE {
        arm_game_over();
    }
}

pub fn note_rewound() {
    REWINDS.fetch_add(1, Ordering::Relaxed);
    REWOUND_NOTICE.store(true, Ordering::Relaxed);
}

pub fn take_rewound_notice() -> bool {
    REWOUND_NOTICE.swap(false, Ordering::Relaxed)
}

pub fn rewind_count() -> u32 {
    REWINDS.load(Ordering::Relaxed)
}
