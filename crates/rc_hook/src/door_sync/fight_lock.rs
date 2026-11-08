use core::ffi::c_void;
use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::Replicant;
use rc_game::fight_doors::{
    FightLockVerdict, HeldLocks, LockOrigin, is_fight_door, judge_door_lock,
};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::door_sync::calls::{applying_from_network, layout_id_of};
use crate::memory::InProcessMemory;
use crate::story_relay::ms_since_an_injected_run_started;

pub(super) static HELD_LOCKS: LazyLock<Mutex<HeldLocks>> = LazyLock::new(Mutex::default);

pub fn lock_is_held_back(build: &VerifiedBuild, door: *mut c_void, argument: i64) -> bool {
    if applying_from_network() {
        return false;
    }
    let door_id = layout_id_of(build, door);
    let Ok(layout_id) = u16::try_from(door_id) else {
        return false;
    };
    if argument == 0 {
        HELD_LOCKS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drop_hold(layout_id);
        return false;
    }
    let phase = Replicant::new(InProcessMemory::new(build))
        .current_phase()
        .unwrap_or_default();
    if !is_fight_door(&phase, door_id) {
        return false;
    }
    let origin = LockOrigin::of_a_local_lock(ms_since_an_injected_run_started());
    let verdict = judge_door_lock(&phase, door_id, true, origin);
    log_fight_lock(&phase, layout_id, verdict, "this game's script");
    if let FightLockVerdict::Hold(_) = verdict {
        HELD_LOCKS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .hold(layout_id);
        return true;
    }
    false
}

pub(super) fn log_fight_lock(phase: &str, layout_id: u16, verdict: FightLockVerdict, from: &str) {
    match verdict {
        FightLockVerdict::Apply => log_line(&format!(
            "doors: fight lock door {layout_id} in {phase} applied (from {from}, this player's own trap)"
        )),
        FightLockVerdict::Hold(reason) => log_line(&format!(
            "doors: fight lock door {layout_id} in {phase} held (from {from}, {})",
            reason.text()
        )),
    }
}
