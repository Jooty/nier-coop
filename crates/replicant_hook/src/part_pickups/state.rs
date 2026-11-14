use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::AtomicPtr;
use std::sync::{Mutex, OnceLock};

use replicant_game::part_pickups::PendingPartPickups;

use crate::build_gate::VerifiedBuild;

pub(super) static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
pub(super) static ORIGINAL_ACTOR_HANDLER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
pub(super) static ORIGINAL_STATE_STEP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
pub(super) static PENDING: Mutex<PendingPartPickups> = Mutex::new(PendingPartPickups::new());
