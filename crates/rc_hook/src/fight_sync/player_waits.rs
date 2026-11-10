use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::PoisonError;
use std::sync::atomic::AtomicPtr;

use rc_game::fight_sync::{PlayerWaitNative, format_player_wait_release, player_wait_ends};
use rc_game::stay_together::FollowRole;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::fight_sync::radio_waits::call_original;
use crate::fight_sync::state::{BUILD, GUEST, here};
use crate::game_hooks::{hook_address, note_yielding_native, slot_holder_name};
use crate::net::net_clock_ms;

static ORIGINAL_PLAYER_WAITS: [AtomicPtr<c_void>; 2] =
    [const { AtomicPtr::new(ptr::null_mut()) }; 2];

pub(super) fn install(build: VerifiedBuild) {
    let detours: [*mut c_void; 2] = [
        default_status_reserve_poll_detour as *const () as *mut c_void,
        on_ground_poll_detour as *const () as *mut c_void,
    ];
    for ((native, detour), original) in PlayerWaitNative::ALL
        .into_iter()
        .zip(detours)
        .zip(&ORIGINAL_PLAYER_WAITS)
    {
        #[expect(
            unsafe_code,
            reason = "hooks the polled waits of two script natives by address"
        )]
        // SAFETY: each RVA starts the wait the script step polls for that native in the verified
        // build (`_WaitPlayerDefaultStatusReserve` 0x70C310, `_WaitPlayerOnGround` 0x70C4C0; docs
        // symbols.tsv): the script manager + 0xD8 in, a byte out (non-zero while it waits).
        unsafe {
            hook_address(
                &format!("LuaNative:{}", native.name()),
                (build.module_base() + u64::from(native.poll_rva())) as usize,
                detour,
                original,
            );
        }
    }
}

extern "system" fn default_status_reserve_poll_detour(args: *mut c_void) -> u8 {
    player_wait_poll(0, args)
}

extern "system" fn on_ground_poll_detour(args: *mut c_void) -> u8 {
    player_wait_poll(1, args)
}

fn player_wait_poll(index: usize, args: *mut c_void) -> u8 {
    let still_waiting = call_original(&ORIGINAL_PLAYER_WAITS[index], args);
    let native = PlayerWaitNative::ALL[index];
    let ends =
        panic::catch_unwind(|| player_wait_polled(native, still_waiting != 0)).unwrap_or(false);
    // Returning 0 is how a polled wait ends: the script step resumes the handler's thread.
    if ends { 0 } else { still_waiting }
}

fn player_wait_polled(native: PlayerWaitNative, still_waiting: bool) -> bool {
    note_yielding_native(native.name());
    if !still_waiting {
        return false;
    }
    let Some(build) = BUILD.get() else {
        return false;
    };
    let Some(here) = here(build).filter(|here| here.role == FollowRole::Guest) else {
        return false;
    };
    let entry_behind_ms = GUEST
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .entry_behind_ms(here.local_phase, net_clock_ms());
    if !player_wait_ends(still_waiting, entry_behind_ms) {
        return false;
    }
    let holder = slot_holder_name().unwrap_or_else(|| "an unknown handler".to_owned());
    log_line(&format_player_wait_release(
        native,
        &holder,
        entry_behind_ms.unwrap_or(0),
    ));
    true
}
