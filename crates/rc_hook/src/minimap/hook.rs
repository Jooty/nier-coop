use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rc_game::minimap::{DEFAULT_PARTNER_ICON_INDEX, MINIMAP_UPDATE_RVA};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::minimap::place::{Outcome, place_partner_icon};

pub(super) static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static INSTALLED: AtomicBool = AtomicBool::new(false);
static DISABLED: AtomicBool = AtomicBool::new(false);
static ICON_INDEX: AtomicI32 = AtomicI32::new(DEFAULT_PARTNER_ICON_INDEX);
static LAST_STATE: AtomicU64 = AtomicU64::new(u64::MAX);
static LAST_VISIBLE_AT_MS: AtomicU64 = AtomicU64::new(u64::MAX);
static CLOCK_START: OnceLock<Instant> = OnceLock::new();
static LAST_HUD: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

const VISIBLE_FRESH_MS: u64 = 500;
const MAP_LOG_CELL: f32 = 0.05;

type MinimapUpdate = unsafe extern "system" fn(*mut c_void);

pub fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(unsafe_code, reason = "hooks the minimap HUD tick by its address")]
    // SAFETY: the RVA starts the minimap HUD update (one HUD-object argument, no result) in the
    // verified build, and the replacement has the same signature.
    let hooked = unsafe {
        hook_address(
            "MinimapHud::Update",
            (build.module_base() + u64::from(MINIMAP_UPDATE_RVA)) as usize,
            minimap_update_detour as *const () as *mut c_void,
            &ORIGINAL_UPDATE,
        )
    };
    INSTALLED.store(hooked, Ordering::Release);
    log_line(&format!(
        "minimap: partner icon detour {}",
        if hooked { "installed" } else { "NOT installed" }
    ));
}

pub fn real_marker_active() -> bool {
    INSTALLED.load(Ordering::Acquire) && !DISABLED.load(Ordering::Acquire)
}

pub fn partner_icon_index() -> i32 {
    ICON_INDEX.load(Ordering::Relaxed)
}

pub fn set_partner_icon_index(index: i32) {
    ICON_INDEX.store(index.clamp(0, 7), Ordering::Relaxed);
}

#[expect(
    unsafe_code,
    reason = "calls the original minimap tick through the trampoline MinHook built"
)]
extern "system" fn minimap_update_detour(hud: *mut c_void) {
    let original = ORIGINAL_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original tick, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, MinimapUpdate>(original)(hud) };
    if DISABLED.load(Ordering::Acquire) {
        return;
    }
    match panic::catch_unwind(|| place_partner_icon(hud)) {
        Ok(Ok(outcome)) => report(hud, outcome),
        Ok(Err(failure)) => disable(&failure.0),
        Err(_) => disable("a panic while placing the icon"),
    }
}

fn disable(reason: &str) {
    if !DISABLED.swap(true, Ordering::AcqRel) {
        log_line(&format!(
            "minimap: partner icon stopped for this run, {reason}; the overlay dot takes over"
        ));
    }
}

fn report(hud: *mut c_void, outcome: Outcome) {
    if LAST_HUD.swap(hud, Ordering::AcqRel) != hud {
        log_line(&format!("minimap: HUD object at {hud:p}"));
    }
    let state = match &outcome {
        Outcome::Skipped(reason) => u64::from(*reason as u8),
        Outcome::Placed(placed) => {
            let cell = |value: f32| ((value / MAP_LOG_CELL).round() as i64 & 0xFFFF) as u64;
            (1 << 63)
                + u64::from(placed.clamped)
                + (u64::from(placed.area) << 1)
                + (cell(placed.partner_map[0]) << 8)
                + (cell(placed.partner_map[1]) << 24)
        }
    };
    if LAST_STATE.swap(state, Ordering::AcqRel) == state {
        return;
    }
    match outcome {
        Outcome::Skipped(reason) => log_line(&format!("minimap: no partner icon ({reason:?})")),
        Outcome::Placed(placed) => log_line(&format!(
            "minimap: partner icon area {} bounds [{:.1} {:.1} {:.1} {:.1}] map ({:.3}, {:.3}) \
             player map ({:.3}, {:.3}) at ({:.1}, {:.1}) {} index {}",
            placed.area,
            placed.bounds[0],
            placed.bounds[1],
            placed.bounds[2],
            placed.bounds[3],
            placed.partner_map[0],
            placed.partner_map[1],
            placed.player_map[0],
            placed.player_map[1],
            placed.x,
            placed.y,
            if placed.clamped {
                "clamped to the edge"
            } else {
                "inside the viewport"
            },
            placed.index,
        )),
    }
}

pub(super) fn note_minimap_shown(shown: bool) {
    let now = CLOCK_START.get_or_init(Instant::now).elapsed();
    let stamp = if shown {
        now.as_millis() as u64
    } else {
        u64::MAX
    };
    LAST_VISIBLE_AT_MS.store(stamp, Ordering::Release);
}

pub fn minimap_shown_recently() -> bool {
    let stamp = LAST_VISIBLE_AT_MS.load(Ordering::Acquire);
    let Some(start) = CLOCK_START.get() else {
        return false;
    };
    stamp != u64::MAX
        && start.elapsed() < Duration::from_millis(stamp.saturating_add(VISIBLE_FRESH_MS))
}
