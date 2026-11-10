use rc_game::Replicant;
use rc_game::downed::{PlayerLock, PlayerLockCall};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::story_jump::call_native_with;

pub(super) fn apply_player_lock(build: &VerifiedBuild, lock: &mut PlayerLock, wanted: bool) {
    let in_play = Replicant::new(InProcessMemory::new(build))
        .current_phase()
        .is_ok();
    let Some(call) = lock.step(wanted, in_play) else {
        return;
    };
    for rva in call.native_rvas() {
        call_native_with(build, *rva, &[]);
    }
    log_line(match call {
        PlayerLockCall::Disable => "downed: player move/attack disabled",
        PlayerLockCall::Enable => "downed: player move/attack enabled",
    });
}
