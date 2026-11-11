use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};

use automata_game::address::{PLAYER_BODY_VTABLE, PLAYER_DAMAGE_FUNCTION};
use automata_game::defeat::{PLAYER_DAMAGE_FUNCTION_START, PLAYER_DAMAGE_VTABLE_OFFSET};
use automata_game::partner::code_starts_with;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::partner_walk_guards::guarded_held_body;

static ORIGINAL_PLAYER_DAMAGE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static FIRST_HIT_KEPT_OFF_LOGGED: AtomicBool = AtomicBool::new(false);
static HITS_KEPT_OFF: AtomicU32 = AtomicU32::new(0);

type PlayerDamage = unsafe extern "system" fn(usize, usize, usize, f32, usize, usize);

/// Keeps this game's hits from the held body's health. False, with nothing hooked, when the
/// function is not the noted one.
pub fn install_partner_harm_guard(calls: GameCalls) -> bool {
    let function = calls.address_value(PLAYER_DAMAGE_FUNCTION);
    let start_matches = calls
        .read_bytes(PLAYER_DAMAGE_FUNCTION, PLAYER_DAMAGE_FUNCTION_START.len())
        .is_ok_and(|found| code_starts_with(&found, &PLAYER_DAMAGE_FUNCTION_START));
    let slot = calls.address_value(PLAYER_BODY_VTABLE) + u64::from(PLAYER_DAMAGE_VTABLE_OFFSET);
    let slot_matches = calls.read_u64_at(slot).is_ok_and(|held| held == function);
    let hooked = start_matches
        && slot_matches
        && {
            #[expect(
                unsafe_code,
                reason = "hooks the body's damage function; the replacement has the signature of the original"
            )]
            // SAFETY: the address is the damage function (vtable slot 151) of the verified build,
            // its start and its vtable slot were just checked, and the replacement forwards all six
            // arguments or returns as the game's own no-damage switch makes the original return.
            unsafe {
                nc_hook::detour::hook_address(
                    "player body damage (this game's hits do not reach the partner body's health)",
                    function as usize,
                    player_damage_detour as *const () as *mut c_void,
                    &ORIGINAL_PLAYER_DAMAGE,
                )
            }
        };
    if !hooked {
        log_line(&format!(
            "partner: the body's damage function is not the noted one or its hook failed (start bytes match: {start_matches}, vtable slot matches: {slot_matches}); this game's hits still lower the partner body's health between two writes of the other player's value"
        ));
    }
    hooked
}

/// How many hits of this game were kept off the held body so far.
pub fn hits_kept_off_the_partner_body() -> u32 {
    HITS_KEPT_OFF.load(Ordering::Relaxed)
}

#[expect(
    unsafe_code,
    reason = "calls the original damage function through the trampoline MinHook built"
)]
extern "system" fn player_damage_detour(
    body: usize,
    amount: usize,
    no_defeat: usize,
    scale: f32,
    fifth: usize,
    sixth: usize,
) {
    if guarded_held_body().is_some_and(|(_, held)| held == body) {
        HITS_KEPT_OFF.fetch_add(1, Ordering::Relaxed);
        if !FIRST_HIT_KEPT_OFF_LOGGED.swap(true, Ordering::Relaxed) {
            log_line(&format!(
                "partner: first hit of this game kept off the partner body's health (amount {}, scale {scale}); its health is the other player's (that the body still plays its hit reaction is a guess until run)",
                amount as u32 as i32
            ));
        }
        return;
    }
    let original = ORIGINAL_PLAYER_DAMAGE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    unsafe {
        core::mem::transmute::<*mut c_void, PlayerDamage>(original)(
            body, amount, no_defeat, scale, fifth, sixth,
        );
    }
}
