use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

use replicant_game::fade_sync::{
    FADE_IN_FUNCTION_RVA, FADE_LAYER_RVA, FADE_OUT_FUNCTION_RVA, HostFadeSync,
};
use replicant_game::stay_together::FollowRole;
use replicant_proto::{FadeSync, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::net::{self, NetState};

static ORIGINAL_FADE_OUT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_FADE_IN: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static MODULE_BASE: AtomicU64 = AtomicU64::new(0);
static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static HOST_SYNC: Mutex<HostFadeSync> = Mutex::new(HostFadeSync::new());
static BUILD: OnceLock<VerifiedBuild> = OnceLock::new();

static LAST_OTHER_LAYER_CALL: Mutex<Option<(u64, bool, u64)>> = Mutex::new(None);

const OTHER_LAYER_NEW_EDGE_GAP_MS: u64 = 500;

type FadeCall = unsafe extern "system" fn(*mut c_void, i32, f32, u32);

pub fn arm_host_zone_change() {
    HOST_SYNC
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .arm(now_ms());
}

pub(super) fn call_fade(fading_out: bool, kind: i32, seconds: f32, depth: u32) -> bool {
    let original = if fading_out {
        ORIGINAL_FADE_OUT.load(Ordering::Acquire)
    } else {
        ORIGINAL_FADE_IN.load(Ordering::Acquire)
    };
    let base = MODULE_BASE.load(Ordering::Relaxed);
    if original.is_null() || base == 0 {
        return false;
    }
    let layer = (base + u64::from(FADE_LAYER_RVA)) as *mut c_void;
    #[expect(
        unsafe_code,
        reason = "calls the game's screen fade through the trampoline MinHook built"
    )]
    // SAFETY: the trampoline continues into the original fade function (layer, kind, seconds,
    // depth); the layer address is the game's fade layer object; this runs on the game thread.
    unsafe {
        core::mem::transmute::<*mut c_void, FadeCall>(original)(layer, kind, seconds, depth);
    }
    true
}

pub(super) fn now_ms() -> u64 {
    STARTED_AT.get_or_init(Instant::now).elapsed().as_millis() as u64
}

pub fn install(build: VerifiedBuild) {
    let base = build.module_base();
    MODULE_BASE.store(base, Ordering::Relaxed);
    let _ = BUILD.set(build);
    #[expect(unsafe_code, reason = "hooks the screen fade functions by address")]
    // SAFETY: both RVAs start the fade out and fade in functions (layer, kind, seconds, depth) in
    // the verified build, and each replacement has the same signature.
    unsafe {
        hook_address(
            "ScreenFade::Out",
            (base + u64::from(FADE_OUT_FUNCTION_RVA)) as usize,
            fade_out_detour as *const () as *mut c_void,
            &ORIGINAL_FADE_OUT,
        );
        hook_address(
            "ScreenFade::In",
            (base + u64::from(FADE_IN_FUNCTION_RVA)) as usize,
            fade_in_detour as *const () as *mut c_void,
            &ORIGINAL_FADE_IN,
        );
    }
}

extern "system" fn fade_out_detour(layer: *mut c_void, kind: i32, seconds: f32, depth: u32) {
    if fade_runs_here(layer, true) {
        run_original(&ORIGINAL_FADE_OUT, layer, kind, seconds, depth);
    }
    let _ = panic::catch_unwind(|| host_sends(layer, true, kind, seconds, depth));
}

extern "system" fn fade_in_detour(layer: *mut c_void, kind: i32, seconds: f32, depth: u32) {
    if fade_runs_here(layer, false) {
        run_original(&ORIGINAL_FADE_IN, layer, kind, seconds, depth);
    }
    let _ = panic::catch_unwind(|| host_sends(layer, false, kind, seconds, depth));
}

fn fade_runs_here(layer: *mut c_void, fading_out: bool) -> bool {
    let base = MODULE_BASE.load(Ordering::Relaxed);
    if layer as u64 != base + u64::from(FADE_LAYER_RVA) {
        return true;
    }
    let Some(build) = BUILD.get() else {
        return true;
    };
    panic::catch_unwind(|| crate::host_view::fade_call_runs(build, fading_out)).unwrap_or(true)
}

#[expect(
    unsafe_code,
    reason = "calls the original fade function through the trampoline MinHook built"
)]
fn run_original(
    original: &AtomicPtr<c_void>,
    layer: *mut c_void,
    kind: i32,
    seconds: f32,
    depth: u32,
) {
    let original = original.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function with this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, FadeCall>(original)(layer, kind, seconds, depth) };
}

fn log_other_layer_edge(layer: u64, fading_out: bool) {
    let now = now_ms();
    let mut last = LAST_OTHER_LAYER_CALL
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let same_call_continuing = last.is_some_and(|(last_layer, last_out, last_ms)| {
        last_layer == layer
            && last_out == fading_out
            && now.saturating_sub(last_ms) < OTHER_LAYER_NEW_EDGE_GAP_MS
    });
    *last = Some((layer, fading_out, now));
    drop(last);
    if !same_call_continuing {
        log_line(&format!(
            "fade: other layer {layer:#x} {}",
            if fading_out { "Out" } else { "In" }
        ));
    }
}

fn host_sends(layer: *mut c_void, fading_out: bool, kind: i32, seconds: f32, depth: u32) {
    let base = MODULE_BASE.load(Ordering::Relaxed);
    if layer as u64 != base + u64::from(FADE_LAYER_RVA) {
        log_other_layer_edge(layer as u64, fading_out);
        return;
    }
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let hosting = FollowRole::from_slot(view.local_slot) == FollowRole::Host
        && view.state == NetState::Connected;
    if !hosting {
        return;
    }
    let due = {
        let mut sync = HOST_SYNC.lock().unwrap_or_else(PoisonError::into_inner);
        if fading_out {
            sync.should_send_fade_out(now_ms())
        } else {
            sync.should_send_fade_in()
        }
    };
    if !due {
        return;
    }
    let Ok(kind) = u8::try_from(kind) else {
        return;
    };
    net.send_relayed(Message::Fade(FadeSync {
        fading_out,
        kind,
        depth,
        seconds,
    }));
    log_line(&format!(
        "fade: host sends fade {} (kind {kind}, {seconds:.2} s, depth {depth:#x})",
        if fading_out { "out" } else { "in" }
    ));
}
