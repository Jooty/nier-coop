use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use replicant_game::blocks::{
    BLOCK_GRAB_REFUSED_LOG_INTERVAL_MS, PARTNER_HOLD_EXPIRY_MS, PLAYER_CAN_GRAB_BLOCK_RVA,
    grab_refused_text,
};
use replicant_game::kaine::{swapped_body_box_grab_refused_text, swapped_body_cannot_move_boxes};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::net::net_clock_ms;
use crate::player_body::real_body;

static ORIGINAL_CAN_GRAB: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static LAST_REFUSAL_LOG_MS: AtomicU64 = AtomicU64::new(0);

struct GateView {
    player: u64,
    partner_held_actors: Vec<(u16, u64)>,
    published_ms: u64,
    object_player: u64,
    partner_held_objects: Vec<u64>,
    objects_published_ms: u64,
}

static GATE: Mutex<GateView> = Mutex::new(GateView {
    player: 0,
    partner_held_actors: Vec::new(),
    published_ms: 0,
    object_player: 0,
    partner_held_objects: Vec::new(),
    objects_published_ms: 0,
});

type CanGrab = unsafe extern "system" fn(*mut c_void, *mut c_void, u32, *mut c_void) -> u8;

pub fn install(build: VerifiedBuild) {
    #[expect(
        unsafe_code,
        reason = "hooks the player's can-grab-block gate by address"
    )]
    // SAFETY: the RVA starts the four-argument gate (rcx player, rdx block, r8d side, r9 out) in the
    // verified build, and it reads no stack argument; the replacement has the same signature.
    unsafe {
        hook_address(
            "Player_CanGrabBlock",
            (build.module_base() + u64::from(PLAYER_CAN_GRAB_BLOCK_RVA)) as usize,
            can_grab_detour as *const () as *mut c_void,
            &ORIGINAL_CAN_GRAB,
        );
    }
}

pub(super) fn publish(player: u64, partner_held_actors: Vec<(u16, u64)>) {
    let mut view = GATE.lock().unwrap_or_else(PoisonError::into_inner);
    view.player = player;
    view.partner_held_actors = partner_held_actors;
    view.published_ms = net_clock_ms();
}

/// The world objects (not blocks) the partner pushes or carries now, refused to this player.
pub(crate) fn publish_partner_held_objects(player: u64, actors: Vec<u64>) {
    let mut view = GATE.lock().unwrap_or_else(PoisonError::into_inner);
    view.object_player = player;
    view.partner_held_objects = actors;
    view.objects_published_ms = net_clock_ms();
}

#[expect(
    unsafe_code,
    reason = "calls the original gate through the trampoline MinHook built"
)]
extern "system" fn can_grab_detour(
    player: *mut c_void,
    block: *mut c_void,
    side: u32,
    out: *mut c_void,
) -> u8 {
    let original = ORIGINAL_CAN_GRAB.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let refused = panic::catch_unwind(|| {
        swapped_body_has_no_box_motions(block)
            || partner_holds_block(player, block)
            || partner_holds_object(player, block)
    });
    if refused.unwrap_or(false) {
        return 0;
    }
    // SAFETY: the trampoline continues into the original gate, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, CanGrab>(original)(player, block, side, out) }
}

fn swapped_body_has_no_box_motions(block: *mut c_void) -> bool {
    if !real_body().is_some_and(|real| swapped_body_cannot_move_boxes(real.body, real.by_swap)) {
        return false;
    }
    log_refusal_unless_logged_lately(&swapped_body_box_grab_refused_text(block as u64));
    true
}

fn log_refusal_unless_logged_lately(text: &str) {
    let now = net_clock_ms();
    let last = LAST_REFUSAL_LOG_MS.load(Ordering::Relaxed);
    if last == 0 || now.saturating_sub(last) >= BLOCK_GRAB_REFUSED_LOG_INTERVAL_MS {
        LAST_REFUSAL_LOG_MS.store(now.max(1), Ordering::Relaxed);
        log_line(text);
    }
}

fn partner_holds_block(player: *mut c_void, block: *mut c_void) -> bool {
    let Ok(view) = GATE.try_lock() else {
        return false;
    };
    let now = net_clock_ms();
    if view.player == 0
        || view.player != player as u64
        || now.saturating_sub(view.published_ms) >= PARTNER_HOLD_EXPIRY_MS
    {
        return false;
    }
    let Some((id, _)) = view
        .partner_held_actors
        .iter()
        .find(|(_, actor)| *actor == block as u64)
    else {
        return false;
    };
    let last = LAST_REFUSAL_LOG_MS.load(Ordering::Relaxed);
    if last == 0 || now.saturating_sub(last) >= BLOCK_GRAB_REFUSED_LOG_INTERVAL_MS {
        LAST_REFUSAL_LOG_MS.store(now.max(1), Ordering::Relaxed);
        log_line(&grab_refused_text(*id));
    }
    true
}

fn partner_holds_object(player: *mut c_void, object: *mut c_void) -> bool {
    let Ok(view) = GATE.try_lock() else {
        return false;
    };
    let now = net_clock_ms();
    if view.object_player == 0
        || view.object_player != player as u64
        || now.saturating_sub(view.objects_published_ms) >= PARTNER_HOLD_EXPIRY_MS
        || !view.partner_held_objects.contains(&(object as u64))
    {
        return false;
    }
    let last = LAST_REFUSAL_LOG_MS.load(Ordering::Relaxed);
    if last == 0 || now.saturating_sub(last) >= BLOCK_GRAB_REFUSED_LOG_INTERVAL_MS {
        LAST_REFUSAL_LOG_MS.store(now.max(1), Ordering::Relaxed);
        log_line(&format!(
            "objects: refused your grab of object {:#x}, the partner holds it",
            object as u64
        ));
    }
    true
}
