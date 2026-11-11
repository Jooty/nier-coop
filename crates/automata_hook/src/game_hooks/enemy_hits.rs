use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::collections::HashMap;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use automata_game::address::{
    ENEMY_DAMAGE_FUNCTION, ENEMY_FLAT_DAMAGE_FUNCTION, ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION,
    GameAddress,
};
use automata_game::enemies::{
    ENEMY_DAMAGE_FUNCTION_START, ENEMY_FLAT_DAMAGE_FUNCTION_START,
    ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION_START, EnemyHit, EnemyId,
    HIT_RECORD_ATTACKER_HANDLE_OFFSET, HitOwner, hit_owner, settle_hit,
};
use automata_game::partner::code_starts_with;
use automata_game::save_redirect::SaveRole;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::partner_walk_guards::guarded_held_body;

const HIT_LINES_LOGGED_AT_MOST: u32 = 60;

static ORIGINAL_ENEMY_DAMAGE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_ENEMY_FLAT_DAMAGE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ENEMY_HIT_CALLS: OnceLock<(GameCalls, SaveRole)> = OnceLock::new();
static SHARED_ENEMIES: Mutex<Option<HashMap<usize, SharedEnemy>>> = Mutex::new(None);
static REPORTED_HITS: Mutex<Vec<EnemyHit>> = Mutex::new(Vec::new());
static HIT_LINES_LEFT: AtomicU32 = AtomicU32::new(HIT_LINES_LOGGED_AT_MOST);

thread_local! {
    static A_HEARD_HIT_IS_BEING_APPLIED: Cell<bool> = const { Cell::new(false) };
}

type EnemyDamage = unsafe extern "system" fn(usize, i32, usize, f32) -> u64;
type EnemyFlatDamage = unsafe extern "system" fn(usize, u32) -> u64;

/// An enemy both copies have, as the last count on this copy found it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedEnemy {
    pub id: EnemyId,
    pub handle: u32,
    pub max_health: i32,
}

/// Hooks the two base damage functions of an enemy; false, with every hit left as the game
/// applies it, when a function start differs from the noted one.
pub fn install_enemy_hits(calls: GameCalls, role: SaveRole) -> bool {
    let starts_match =
        start_is_the_noted_one(&calls, ENEMY_DAMAGE_FUNCTION, &ENEMY_DAMAGE_FUNCTION_START)
            && start_is_the_noted_one(
                &calls,
                ENEMY_FLAT_DAMAGE_FUNCTION,
                &ENEMY_FLAT_DAMAGE_FUNCTION_START,
            )
            && start_is_the_noted_one(
                &calls,
                ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION,
                &ENEMY_FORCED_DEFEAT_STATE_TEST_FUNCTION_START,
            );
    let hooked = starts_match
        && ENEMY_HIT_CALLS.set((calls, role)).is_ok()
        && {
            #[expect(
                unsafe_code,
                reason = "hooks an enemy's damage function; the replacement has the signature of the original"
            )]
            // SAFETY: the address is the base enemy damage function of the verified build and
            // its start was just checked; the replacement forwards all four arguments.
            unsafe {
                nc_hook::detour::hook_address(
                    "enemy damage (whose hit counts on a shared enemy)",
                    calls.address_value(ENEMY_DAMAGE_FUNCTION) as usize,
                    enemy_damage_detour as *const () as *mut c_void,
                    &ORIGINAL_ENEMY_DAMAGE,
                )
            }
        }
        && {
            #[expect(
                unsafe_code,
                reason = "hooks an enemy's flat damage function; the replacement has the signature of the original"
            )]
            // SAFETY: the address is the base enemy flat damage function of the verified build
            // and its start was just checked; the replacement forwards both arguments.
            unsafe {
                nc_hook::detour::hook_address(
                    "enemy flat damage (whose hit counts on a shared enemy)",
                    calls.address_value(ENEMY_FLAT_DAMAGE_FUNCTION) as usize,
                    enemy_flat_damage_detour as *const () as *mut c_void,
                    &ORIGINAL_ENEMY_FLAT_DAMAGE,
                )
            }
        };
    if !hooked {
        log_line(
            "enemies: the damage functions are not hooked, enemy health and defeat are not shared",
        );
    }
    hooked
}

/// The enemies whose hits are settled between the copies, by the address of their body; none
/// leaves every hit as the game applies it.
pub fn set_shared_enemies(shared: HashMap<usize, SharedEnemy>) {
    if let Ok(mut held) = SHARED_ENEMIES.lock() {
        *held = (!shared.is_empty()).then_some(shared);
    }
}

/// Runs the host's own call of the damage code for a hit the guest reported; the hooks leave
/// that call alone.
pub fn while_a_heard_hit_is_applied<R>(apply: impl FnOnce() -> R) -> R {
    let before = A_HEARD_HIT_IS_BEING_APPLIED.replace(true);
    let result = apply();
    A_HEARD_HIT_IS_BEING_APPLIED.set(before);
    result
}

/// The local player's hits on shared enemies since the last call; the guest sends them to the host.
pub fn take_reported_enemy_hits() -> Vec<EnemyHit> {
    REPORTED_HITS
        .lock()
        .map(|mut hits| core::mem::take(&mut *hits))
        .unwrap_or_default()
}

#[expect(
    unsafe_code,
    reason = "calls the original enemy damage function through the trampoline MinHook built"
)]
extern "system" fn enemy_damage_detour(enemy: usize, power: i32, record: usize, scale: f32) -> u64 {
    let original = ORIGINAL_ENEMY_DAMAGE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let shared = shared_enemy_before_its_hit(enemy);
    // SAFETY: the trampoline continues into the original function with the arguments received.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, EnemyDamage>(original)(enemy, power, record, scale)
    };
    if let Some((shared, before)) = shared {
        settle_the_hit(enemy, shared, before, attacker_of(record));
    }
    result
}

#[expect(
    unsafe_code,
    reason = "calls the original enemy flat damage function through the trampoline MinHook built"
)]
extern "system" fn enemy_flat_damage_detour(enemy: usize, amount: u32) -> u64 {
    let original = ORIGINAL_ENEMY_FLAT_DAMAGE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let shared = shared_enemy_before_its_hit(enemy);
    // SAFETY: the trampoline continues into the original function with the arguments received.
    let result =
        unsafe { core::mem::transmute::<*mut c_void, EnemyFlatDamage>(original)(enemy, amount) };
    if let Some((shared, before)) = shared {
        settle_the_hit(enemy, shared, before, None);
    }
    result
}

fn start_is_the_noted_one(calls: &GameCalls, function: GameAddress, noted_start: &[u8]) -> bool {
    calls
        .read_bytes(function, noted_start.len())
        .is_ok_and(|found| code_starts_with(&found, noted_start))
}

fn shared_enemy_before_its_hit(enemy: usize) -> Option<(SharedEnemy, i32)> {
    if A_HEARD_HIT_IS_BEING_APPLIED.get() {
        return None;
    }
    let (calls, _) = ENEMY_HIT_CALLS.get()?;
    let shared = *SHARED_ENEMIES.lock().ok()?.as_ref()?.get(&enemy)?;
    // The count is a moment old: the body must still belong to the entity it was counted with.
    if calls.entity_handle_of_body(enemy) != Some(shared.handle) {
        return None;
    }
    Some((shared, calls.read_enemy_health(enemy)?))
}

fn attacker_of(record: usize) -> Option<u32> {
    if record == 0 {
        return None;
    }
    let (calls, _) = ENEMY_HIT_CALLS.get()?;
    calls
        .read_u32_at(record as u64 + u64::from(HIT_RECORD_ATTACKER_HANDLE_OFFSET))
        .ok()
}

fn settle_the_hit(enemy: usize, shared: SharedEnemy, before: i32, attacker: Option<u32>) {
    let Some((calls, role)) = ENEMY_HIT_CALLS.get() else {
        return;
    };
    let Some(after) = calls.read_enemy_health(enemy) else {
        return;
    };
    let partner_body_handle =
        guarded_held_body().and_then(|(_, body)| calls.entity_handle_of_body(body));
    let owner = hit_owner(attacker, partner_body_handle);
    let settlement = settle_hit(*role, owner, before, after);
    if let Some(health) = settlement.health_written {
        calls.write_enemy_health(enemy, health);
    }
    if settlement.amount_reported > 0
        && let Ok(mut hits) = REPORTED_HITS.lock()
    {
        hits.push(EnemyHit {
            id: shared.id,
            amount: settlement.amount_reported,
            max_health: shared.max_health,
        });
    }
    if after < before {
        log_the_hit(
            calls,
            shared,
            before,
            after,
            attacker,
            owner,
            settlement.health_written,
        );
    }
}

fn log_the_hit(
    calls: &GameCalls,
    shared: SharedEnemy,
    before: i32,
    after: i32,
    attacker: Option<u32>,
    owner: HitOwner,
    health_written: Option<i32>,
) {
    let took_a_line = HIT_LINES_LEFT
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |left| {
            left.checked_sub(1)
        })
        .is_ok();
    if !took_a_line {
        return;
    }
    let attacker_text = match attacker {
        Some(handle) => match calls.object_id_of_handle(handle) {
            Some(object_id) => format!("attacker object {object_id:#X}"),
            None => format!("attacker handle {handle:#X} with no body"),
        },
        None => "no hit record".to_owned(),
    };
    let owner_text = match owner {
        HitOwner::LocalPlayer => "counted as this copy's",
        HitOwner::PartnerBody => "the partner body's, taken back",
    };
    let kept_text =
        health_written.map_or(String::new(), |health| format!(", health set to {health}"));
    log_line(&format!(
        "enemies: hit on {}: health {before} -> {after}, {attacker_text}, {owner_text}{kept_text}",
        shared.id.label()
    ));
}
