use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use automata_game::address::{
    ACTION_UPDATE_FUNCTION, CAMERA_MODE_REQUEST_FLAG_WORD_VALUE, CAMERA_OBJECT,
};
use automata_game::partner::{
    ACTION_UPDATE_FUNCTION_START, BODY_BASE_HEALTH_OFFSET,
    CAMERA_FIELDS_A_PLAYER_BODY_UPDATE_WRITES_OFFSETS, CAMERA_LOCK_TARGET_HANDLES_OFFSET,
    CAMERA_MODE_OFFSET, CAMERA_WORD_ZEROED_BY_A_MODE_REQUEST_OFFSET, CameraKept, code_starts_with,
};
use nc_hook::diagnostics::log_line;
use nc_hook::memory::MemoryError;

use crate::game_calls::GameCalls;
use crate::game_hooks::partner_walk_guards::body_is_held;

static ORIGINAL_ACTION_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static CAMERA_CALLS: OnceLock<GameCalls> = OnceLock::new();
static FIRST_CAMERA_KEEP_LOGGED: AtomicBool = AtomicBool::new(false);
static LOW_BASE_HEALTH_LOGGED: AtomicBool = AtomicBool::new(false);

type ActionUpdate = unsafe extern "system" fn(usize) -> usize;

/// Keeps the local camera mode and lock target from the held body's own update; false, with
/// nothing hooked, when the function start differs.
pub fn install_camera_guard(calls: GameCalls) -> bool {
    let start_matches = calls
        .read_bytes(ACTION_UPDATE_FUNCTION, ACTION_UPDATE_FUNCTION_START.len())
        .is_ok_and(|found| code_starts_with(&found, &ACTION_UPDATE_FUNCTION_START));
    let hooked = start_matches
        && CAMERA_CALLS.set(calls).is_ok()
        && {
            #[expect(
                unsafe_code,
                reason = "hooks the body's action update; the replacement has the signature of the original"
            )]
            // SAFETY: the address is the action update (vtable slot 8) of the verified build and
            // its start was just checked; the replacement forwards the one argument and the result.
            unsafe {
                nc_hook::detour::hook_address(
                    "player body action update (keeps the local camera from the partner body)",
                    calls.address_value(ACTION_UPDATE_FUNCTION) as usize,
                    action_update_detour as *const () as *mut c_void,
                    &ORIGINAL_ACTION_UPDATE,
                )
            }
        };
    if !hooked {
        log_line(
            "partner: the body's action update is not hooked, so the camera mode and the lock target may follow the partner body",
        );
    }
    hooked
}

#[expect(
    unsafe_code,
    reason = "calls the original action update through the trampoline MinHook built"
)]
extern "system" fn action_update_detour(body: usize) -> usize {
    let original = ORIGINAL_ACTION_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let kept = camera_values_if_this_is_the_held_body(body);
    // SAFETY: the trampoline continues into the original function with the argument received.
    let result = unsafe { core::mem::transmute::<*mut c_void, ActionUpdate>(original)(body) };
    if let Some((calls, kept)) = kept {
        let put_back = put_camera_values_back(&calls, &kept);
        if !FIRST_CAMERA_KEEP_LOGGED.swap(true, Ordering::Relaxed) {
            log_line(&format!(
                "partner: the camera mode ({}) and lock target are put back after the partner body's update (written {}; that this keeps the camera on the local player is a guess until run)",
                kept.mode,
                put_back.is_ok()
            ));
        }
    }
    result
}

fn camera_values_if_this_is_the_held_body(body: usize) -> Option<(GameCalls, CameraKept)> {
    if !body_is_held(body) {
        return None;
    }
    let calls = *CAMERA_CALLS.get()?;
    log_low_base_health_once(&calls, body);
    let camera = calls.address_value(CAMERA_OBJECT);
    let mut other_fields = [0; CAMERA_FIELDS_A_PLAYER_BODY_UPDATE_WRITES_OFFSETS.len()];
    for (field, offset) in other_fields
        .iter_mut()
        .zip(CAMERA_FIELDS_A_PLAYER_BODY_UPDATE_WRITES_OFFSETS)
    {
        *field = calls.read_u32_at(camera + u64::from(offset)).ok()?;
    }
    let kept = CameraKept {
        other_fields,
        lock_targets: calls
            .read_u32s_at(camera + u64::from(CAMERA_LOCK_TARGET_HANDLES_OFFSET))
            .ok()?,
        mode: calls
            .read_u32_at(camera + u64::from(CAMERA_MODE_OFFSET))
            .ok()?,
        word_zeroed_by_a_mode_request: calls
            .read_u32_at(camera + u64::from(CAMERA_WORD_ZEROED_BY_A_MODE_REQUEST_OFFSET))
            .ok()?,
        flag_word: calls.read_u32(CAMERA_MODE_REQUEST_FLAG_WORD_VALUE).ok()?,
    };
    Some((calls, kept))
}

fn log_low_base_health_once(calls: &GameCalls, body: usize) {
    let base_health = calls.read_i32_at(calls.body_field(body, BODY_BASE_HEALTH_OFFSET));
    if base_health.is_ok_and(|health| health < 1)
        && !LOW_BASE_HEALTH_LOGGED.swap(true, Ordering::Relaxed)
    {
        log_line(
            "partner: the held body's base health is below 1, so its update sets a global the hero's update sets at death (what reads it was not found)",
        );
    }
}

fn put_camera_values_back(calls: &GameCalls, kept: &CameraKept) -> Result<(), MemoryError> {
    let camera = calls.address_value(CAMERA_OBJECT);
    for (field, offset) in kept
        .other_fields
        .iter()
        .zip(CAMERA_FIELDS_A_PLAYER_BODY_UPDATE_WRITES_OFFSETS)
    {
        calls.write_u32_at(camera + u64::from(offset), *field)?;
    }
    calls.write_u32s_at(
        camera + u64::from(CAMERA_LOCK_TARGET_HANDLES_OFFSET),
        &kept.lock_targets,
    )?;
    calls.write_u32_at(camera + u64::from(CAMERA_MODE_OFFSET), kept.mode)?;
    calls.write_u32_at(
        camera + u64::from(CAMERA_WORD_ZEROED_BY_A_MODE_REQUEST_OFFSET),
        kept.word_zeroed_by_a_mode_request,
    )?;
    let flag_word = calls.read_u32(CAMERA_MODE_REQUEST_FLAG_WORD_VALUE)?;
    calls.write_u32(
        CAMERA_MODE_REQUEST_FLAG_WORD_VALUE,
        kept.flag_word_put_back(flag_word),
    )
}
