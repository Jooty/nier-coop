use std::sync::atomic::AtomicU32;
use std::sync::{Mutex, OnceLock};

use rc_game::script_rolls::{RollOrdinals, SharedRollStore};

use crate::build_gate::VerifiedBuild;

pub(super) static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
pub(super) static ORDINALS: Mutex<RollOrdinals> = Mutex::new(RollOrdinals::new());
pub(super) static STORE: Mutex<SharedRollStore> = Mutex::new(SharedRollStore::new());
pub(super) static ROLLS_SENT: AtomicU32 = AtomicU32::new(0);
pub(super) static ROLLS_USED: AtomicU32 = AtomicU32::new(0);
pub(super) static ROLLS_MISSED: AtomicU32 = AtomicU32::new(0);
