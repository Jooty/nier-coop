use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, AtomicU32, AtomicUsize, Ordering};

use automata_game::address::{PLAYER_BODY_VTABLE, PLAYER_DAMAGE_FUNCTION};
use automata_game::defeat::{PLAYER_DAMAGE_FUNCTION_START, PLAYER_DAMAGE_VTABLE_OFFSET};
use automata_game::downed::{
    BODY_FLIGHT_FLAG_OFFSETS, BODY_OWN_DEFEAT_FLOOR_MODE_OFFSET,
    BODY_OWN_DEFEAT_FLOOR_SWITCH_OFFSET, HeroGuard, HeroHit, HeroHitTreatment, hero_hit_treatment,
    hit_put_the_hero_down,
};
use automata_game::partner::{BODY_BERSERK_SWITCH_OFFSET, BODY_HEALTH_OFFSET, code_starts_with};
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::partner_walk_guards::body_is_held;

static ORIGINAL_PLAYER_DAMAGE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static FIRST_HIT_KEPT_OFF_LOGGED: AtomicBool = AtomicBool::new(false);
static HITS_KEPT_OFF: AtomicU32 = AtomicU32::new(0);
static HERO_GUARD_CALLS: OnceLock<GameCalls> = OnceLock::new();
static GUARDED_HERO: AtomicUsize = AtomicUsize::new(0);
static HERO_GUARD: AtomicU8 = AtomicU8::new(HeroGuard::Off.as_number());
static A_HIT_LEFT_THE_HERO_DOWN: AtomicBool = AtomicBool::new(false);
static FIRST_HIT_KEPT_OFF_THE_DOWNED_HERO_LOGGED: AtomicBool = AtomicBool::new(false);
static FIRST_HIT_ON_A_FLYING_HERO_LOGGED: AtomicBool = AtomicBool::new(false);
static HERO_DEFEATS_HELD_OFF: AtomicU32 = AtomicU32::new(0);

const HERO_DEFEATS_HELD_OFF_LOGGED_AT_MOST: u32 = 40;
const NO_DEFEAT_ASKED: usize = 1;

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
    let _ = HERO_GUARD_CALLS.set(calls);
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

/// What the hook does with hits on the local hero from now on; game thread only.
pub fn guard_the_hero(hero: Option<usize>, guard: HeroGuard) {
    GUARDED_HERO.store(hero.unwrap_or(0), Ordering::Relaxed);
    let guard = if hero.is_some() {
        guard
    } else {
        HeroGuard::Off
    };
    HERO_GUARD.store(guard.as_number(), Ordering::Relaxed);
}

/// True once after a hit that only the hold kept from defeating the hero.
pub fn take_hit_that_left_the_hero_down() -> bool {
    A_HIT_LEFT_THE_HERO_DOWN.swap(false, Ordering::Relaxed)
}

/// How many hits of this game were kept off the held body so far.
pub fn hits_kept_off_the_partner_body() -> u32 {
    HITS_KEPT_OFF.load(Ordering::Relaxed)
}

extern "system" fn player_damage_detour(
    body: usize,
    amount: usize,
    no_defeat: usize,
    scale: f32,
    fifth: usize,
    sixth: usize,
) {
    if body_is_held(body) {
        HITS_KEPT_OFF.fetch_add(1, Ordering::Relaxed);
        if !FIRST_HIT_KEPT_OFF_LOGGED.swap(true, Ordering::Relaxed) {
            log_line(&format!(
                "partner: first hit of this game kept off the partner body's health (amount {}, scale {scale}); its health is the other player's (that the body still plays its hit reaction is a guess until run)",
                amount as u32 as i32
            ));
        }
        return;
    }
    if hit_on_the_guarded_hero_was_handled(body, amount, no_defeat, scale, fifth, sixth) {
        return;
    }
    call_original_damage(body, amount, no_defeat, scale, fifth, sixth);
}

fn hit_on_the_guarded_hero_was_handled(
    body: usize,
    amount: usize,
    no_defeat: usize,
    scale: f32,
    fifth: usize,
    sixth: usize,
) -> bool {
    let guard = HeroGuard::from_number(HERO_GUARD.load(Ordering::Relaxed));
    if guard == HeroGuard::Off || GUARDED_HERO.load(Ordering::Relaxed) != body {
        return false;
    }
    let Some(calls) = HERO_GUARD_CALLS.get() else {
        return false;
    };
    let int = |offset| {
        calls
            .read_i32_at(calls.body_field(body, offset))
            .unwrap_or(0)
    };
    let hit = HeroHit {
        // The game reads the argument as 32 bits.
        caller_asks_for_no_defeat: no_defeat as u32 != 0,
        berserk_switch: int(BODY_BERSERK_SWITCH_OFFSET),
        own_floor_switch: int(BODY_OWN_DEFEAT_FLOOR_SWITCH_OFFSET),
        own_floor_mode: int(BODY_OWN_DEFEAT_FLOOR_MODE_OFFSET),
        flight_flags: BODY_FLIGHT_FLAG_OFFSETS.map(int),
    };
    match hero_hit_treatment(guard, hit) {
        HeroHitTreatment::Untouched => {
            if hit.hero_flies() && !FIRST_HIT_ON_A_FLYING_HERO_LOGGED.swap(true, Ordering::Relaxed)
            {
                log_line(&format!(
                    "downed: a hit on the hero while its flight flags are set ({:?}); its defeat is not held off there, the downed pose is a state of a hero on foot (that the flags mean flying is a guess; logged once)",
                    hit.flight_flags
                ));
            }
            false
        }
        HeroHitTreatment::KeptOff => {
            if !FIRST_HIT_KEPT_OFF_THE_DOWNED_HERO_LOGGED.swap(true, Ordering::Relaxed) {
                log_line(&format!(
                    "downed: first hit kept off the health of the hero that is down or was just revived (amount {}, scale {scale}; logged once)",
                    amount as u32 as i32
                ));
            }
            true
        }
        HeroHitTreatment::Floored => {
            let before = int(BODY_HEALTH_OFFSET);
            call_original_damage(body, amount, NO_DEFEAT_ASKED, scale, fifth, sixth);
            let after = int(BODY_HEALTH_OFFSET);
            if hit_put_the_hero_down(hit, before, after) {
                A_HIT_LEFT_THE_HERO_DOWN.store(true, Ordering::Relaxed);
                let count = HERO_DEFEATS_HELD_OFF.fetch_add(1, Ordering::Relaxed);
                if count < HERO_DEFEATS_HELD_OFF_LOGGED_AT_MOST {
                    log_line(&format!(
                        "downed: a hit that would have defeated the hero was held off with the game's own no-defeat argument (amount {}, scale {scale}, health {before} -> {after})",
                        amount as u32 as i32
                    ));
                }
            }
            true
        }
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original damage function through the trampoline MinHook built"
)]
fn call_original_damage(
    body: usize,
    amount: usize,
    no_defeat: usize,
    scale: f32,
    fifth: usize,
    sixth: usize,
) {
    let original = ORIGINAL_PLAYER_DAMAGE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received,
    // or with the no-defeat argument the game's own callers also pass.
    unsafe {
        core::mem::transmute::<*mut c_void, PlayerDamage>(original)(
            body, amount, no_defeat, scale, fifth, sixth,
        );
    }
}
