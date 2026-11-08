use core::ffi::{CStr, c_void};
use core::ptr;
use std::collections::HashSet;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{LazyLock, Mutex, PoisonError};

use rc_game::offsets::{
    CHANGE_MAP_NATIVE_FUNCTION_RVA, CHANGE_PHASE_NATIVE_FUNCTION_RVA,
    POP_FIRED_CALLBACK_FUNCTION_RVA, SELECT_WARP_POINT_NATIVE_FUNCTION_RVA,
    TRIGGER_CALLBACK_NAME_CAPACITY,
};
use rc_game::zone_rules::{ZoneChangeSource, is_zone_exit_callback};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::door_walk::take_recent_door_use;
use crate::game_hooks::hook_slot::hook_address;
use crate::game_hooks::teleport::request_teleport;
use crate::story_relay::suppress_fired_call;
use crate::zone_rules::veto_zone_change;

const MAX_SEEN_CALLBACK_NAMES: usize = 400;

static SEEN_CALLBACKS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Mutex::default);
static ORIGINAL_POP_FIRED_CALLBACK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_CHANGE_MAP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_CHANGE_PHASE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SELECT_WARP_POINT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type PopFiredCallback = unsafe extern "system" fn(*mut c_void, *mut u8, *mut *mut c_void) -> u64;
type SelectWarpPoint = unsafe extern "system" fn(i32, i32);
type LuaNative = unsafe extern "system" fn(*mut c_void);

pub(super) fn install(build: VerifiedBuild) {
    let base = build.module_base();
    #[expect(unsafe_code, reason = "hooks the zone change entry points by address")]
    // SAFETY: each RVA starts the function the replacement mirrors in the verified build:
    // TriggerManager pop of a fired callback, and the _ChangeMap and _ChangePhase Lua natives.
    unsafe {
        hook_address(
            "TriggerManager_PopFiredCallback",
            (base + u64::from(POP_FIRED_CALLBACK_FUNCTION_RVA)) as usize,
            pop_fired_callback_detour as *const () as *mut c_void,
            &ORIGINAL_POP_FIRED_CALLBACK,
        );
        hook_address(
            "ChangeMap_native",
            (base + u64::from(CHANGE_MAP_NATIVE_FUNCTION_RVA)) as usize,
            change_map_detour as *const () as *mut c_void,
            &ORIGINAL_CHANGE_MAP,
        );
        hook_address(
            "ChangePhase_native",
            (base + u64::from(CHANGE_PHASE_NATIVE_FUNCTION_RVA)) as usize,
            change_phase_detour as *const () as *mut c_void,
            &ORIGINAL_CHANGE_PHASE,
        );
        hook_address(
            "SelectWarpPoint_native",
            (base + u64::from(SELECT_WARP_POINT_NATIVE_FUNCTION_RVA)) as usize,
            select_warp_point_detour as *const () as *mut c_void,
            &ORIGINAL_SELECT_WARP_POINT,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original pop function and reads the name buffer it filled"
)]
extern "system" fn pop_fired_callback_detour(
    manager: *mut c_void,
    name_out: *mut u8,
    argument_out: *mut *mut c_void,
) -> u64 {
    let original = ORIGINAL_POP_FIRED_CALLBACK.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with this exact signature.
    let fired = unsafe {
        core::mem::transmute::<*mut c_void, PopFiredCallback>(original)(
            manager,
            name_out,
            argument_out,
        )
    };
    if fired & 0xFF == 0 || name_out.is_null() {
        return fired;
    }
    // SAFETY: the original just wrote a NUL-terminated name into the caller's 0x40 byte buffer.
    let name = unsafe { callback_name(name_out) };
    note_callback_seen(&name);
    crate::cutscene_skip::note_fired_callback(&name);
    crate::cutscene_skip::note_popped_callback(&name);
    if crate::cutscene_skip::is_injected_duplicate(&name) {
        return 0;
    }
    if crate::cutscene_skip::hold_fired_callback(&name) {
        return 0;
    }
    if is_zone_exit_callback(&name)
        && vetoed(
            &format!("exit callback {name}"),
            ZoneChangeSource::PlayerChosen,
        )
    {
        put_player_back_at_the_door();
        return 0;
    }
    // SAFETY: the original just stored the argument block pointer through this out pointer.
    let arguments = if argument_out.is_null() {
        0
    } else {
        unsafe { *argument_out as u64 }
    };
    if suppress_fired_call(&name, arguments) {
        return 0;
    }
    fired
}

#[expect(unsafe_code, reason = "reads the callback name buffer")]
unsafe fn callback_name(name_out: *const u8) -> String {
    // SAFETY: the buffer holds TRIGGER_CALLBACK_NAME_CAPACITY bytes and ends in NUL (strcpy_s).
    let bytes = unsafe { core::slice::from_raw_parts(name_out, TRIGGER_CALLBACK_NAME_CAPACITY) };
    CStr::from_bytes_until_nul(bytes)
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn put_player_back_at_the_door() {
    let Some(position) = take_recent_door_use() else {
        return;
    };
    request_teleport(position);
    log_line(&format!(
        "zone gate: refused after the walk, moved back to {:.1}, {:.1}, {:.1}",
        position.x, position.y, position.z
    ));
}

fn vetoed(path: &str, source: ZoneChangeSource) -> bool {
    panic::catch_unwind(|| veto_zone_change(path, source)).unwrap_or(false)
}

extern "system" fn change_map_detour(lua_state: *mut c_void) {
    run_native_unless_vetoed(&ORIGINAL_CHANGE_MAP, "_ChangeMap native", lua_state);
}

extern "system" fn change_phase_detour(lua_state: *mut c_void) {
    run_native_unless_vetoed(&ORIGINAL_CHANGE_PHASE, "_ChangePhase native", lua_state);
}

#[expect(
    unsafe_code,
    reason = "calls the original Lua native through the trampoline MinHook built"
)]
fn run_native_unless_vetoed(original: &AtomicPtr<c_void>, path: &str, lua_state: *mut c_void) {
    let original = original.load(Ordering::Acquire);
    if original.is_null() || vetoed(path, ZoneChangeSource::Scripted) {
        return;
    }
    // SAFETY: the trampoline continues into the original native, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, LuaNative>(original)(lua_state) };
}

#[expect(
    unsafe_code,
    reason = "calls the original warp selection through the trampoline MinHook built"
)]
extern "system" fn select_warp_point_detour(point: i32, other: i32) {
    let original = ORIGINAL_SELECT_WARP_POINT.load(Ordering::Acquire);
    if original.is_null() || vetoed("warp menu", ZoneChangeSource::PlayerChosen) {
        return;
    }
    // SAFETY: the trampoline continues into the original function, which takes these two ints.
    unsafe { core::mem::transmute::<*mut c_void, SelectWarpPoint>(original)(point, other) };
}

fn note_callback_seen(name: &str) {
    let mut seen = SEEN_CALLBACKS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if seen.len() < MAX_SEEN_CALLBACK_NAMES && seen.insert(name.to_owned()) {
        log_line(&format!("zone rules: callback fired: {name}"));
    }
}
