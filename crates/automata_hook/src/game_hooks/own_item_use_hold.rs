use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

use automata_game::address::{
    AUTO_ITEM_USE_CHIP_CHECK_FUNCTION, AUTO_ITEM_USE_HEALTH_SHARE_VALUE, CHIP_LEVEL_FUNCTION,
    GameAddress, USE_ITEM_FUNCTION,
};
use automata_game::downed::{
    AUTO_ITEM_USE_CHIP_CHECK_FUNCTION_START, AUTO_ITEM_USE_CHIP_TYPE, AUTO_ITEM_USE_HEALTH_SHARE,
    AutoItemUseHold, AutoItemUseReading, BODY_AUTO_ITEM_USE_COOL_DOWN_OFFSET, BODY_STEP_OFFSET,
    CHIP_LEVEL_FUNCTION_START, ITEM_USES_HELD_LOGGED_AT_MOST, ItemUseAsker,
    USE_ITEM_FUNCTION_START, auto_item_use_check_is_skipped, item_use_asker, item_use_is_held,
};
use automata_game::exe::ExeVariant;
use automata_game::partner::{BODY_HEALTH_OFFSET, code_starts_with};
use nc_hook::diagnostics::log_line;
use windows_sys::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;

use crate::game_calls::GameCalls;
use crate::game_hooks::partner_walk_guards::guarded_held_body;

static ORIGINAL_USE_ITEM: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_AUTO_ITEM_USE_CHECK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOLD_CONTEXT: OnceLock<HoldContext> = OnceLock::new();
static HELD_HERO: AtomicUsize = AtomicUsize::new(0);
static AUTO_CHECK_HOLD: Mutex<AutoItemUseHold> = Mutex::new(AutoItemUseHold::new());
static AUTOMATIC_USES_HELD: AtomicU32 = AtomicU32::new(0);
static OTHER_USES_HELD: AtomicU32 = AtomicU32::new(0);
static HELD_USE_LINES_LOGGED: AtomicU32 = AtomicU32::new(0);
static HOLD_WAS_ON: AtomicBool = AtomicBool::new(false);
static PARTNER_BODY_CHECKS_SKIPPED: AtomicU32 = AtomicU32::new(0);
static FIRST_PARTNER_BODY_CHECK_SKIP_LOGGED: AtomicBool = AtomicBool::new(false);

const CALLER_SEARCH_FRAMES: usize = 8;
const GAME_IMAGE_RVA_LIMIT: u64 = 0x200_0000;
const NOT_USED: u64 = 0;

type UseItem = unsafe extern "system" fn(usize, i32, f32) -> u64;
type AutoItemUseCheck = unsafe extern "system" fn(usize);
type ChipLevel = unsafe extern "system" fn(usize, i32) -> i32;

#[derive(Debug, Clone, Copy)]
struct HoldContext {
    calls: GameCalls,
    variant: ExeVariant,
    chip_level_is_trusted: bool,
    health_share: f32,
}

/// Hooks the game's use-item function and its Auto-Use Item chip check so that a downed hero's
/// own game uses no recovery item on it. False, with nothing hooked, when neither is the noted one.
pub fn install_own_item_use_hold(calls: GameCalls, variant: ExeVariant) -> bool {
    let starts_with = |address, start: &[u8]| {
        calls
            .read_bytes(address, start.len())
            .is_ok_and(|found| code_starts_with(&found, start))
    };
    let use_item_matches = starts_with(USE_ITEM_FUNCTION, &USE_ITEM_FUNCTION_START);
    let check_matches = starts_with(
        AUTO_ITEM_USE_CHIP_CHECK_FUNCTION,
        &AUTO_ITEM_USE_CHIP_CHECK_FUNCTION_START,
    );
    let chip_level_is_trusted = starts_with(CHIP_LEVEL_FUNCTION, &CHIP_LEVEL_FUNCTION_START);
    let health_share = calls
        .read_bytes(AUTO_ITEM_USE_HEALTH_SHARE_VALUE, 4)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .map(f32::from_le_bytes)
        .filter(|share| share.is_finite() && *share > 0.0 && *share <= 1.0)
        .unwrap_or(AUTO_ITEM_USE_HEALTH_SHARE);
    let _ = HOLD_CONTEXT.set(HoldContext {
        calls,
        variant,
        chip_level_is_trusted,
        health_share,
    });
    let use_item_hooked = use_item_matches
        && hook(
            calls,
            USE_ITEM_FUNCTION,
            "use item (a downed hero's own game uses no recovery item on it)",
            use_item_detour as *const () as *mut c_void,
            &ORIGINAL_USE_ITEM,
        );
    let check_hooked = check_matches
        && hook(
            calls,
            AUTO_ITEM_USE_CHIP_CHECK_FUNCTION,
            "Auto-Use Item chip check (held on a downed hero)",
            auto_item_use_check_detour as *const () as *mut c_void,
            &ORIGINAL_AUTO_ITEM_USE_CHECK,
        );
    log_line(&format!(
        "downed: a downed hero's own item use is held: use-item function hooked: {use_item_hooked} (start bytes match: {use_item_matches}), Auto-Use Item chip check hooked: {check_hooked} (start bytes match: {check_matches}), chip level function is the noted one: {chip_level_is_trusted}, the chip's health share: {health_share}"
    ));
    use_item_hooked || check_hooked
}

/// How many times the Auto-Use Item chip check was skipped on the partner body so far.
pub fn auto_item_use_checks_skipped_on_the_partner_body() -> u32 {
    PARTNER_BODY_CHECKS_SKIPPED.load(Ordering::Relaxed)
}

/// The hero whose own item use is held from now on, or None; game thread only.
pub fn hold_own_item_use(hero: Option<usize>) {
    HELD_HERO.store(hero.unwrap_or(0), Ordering::Relaxed);
    let on = hero.is_some();
    if HOLD_WAS_ON.swap(on, Ordering::Relaxed) == on {
        return;
    }
    if on {
        log_line(
            "downed: the hero's own recovery item use is held while it is down (the Auto-Use Item chip, the quick item menu, the menus, a script command)",
        );
        return;
    }
    AUTO_CHECK_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .forget();
    log_line(&format!(
        "downed: the hold on the hero's own item use is lifted; held during that down: {} automatic uses, {} other uses (the item count is as it was at the down)",
        AUTOMATIC_USES_HELD.swap(0, Ordering::Relaxed),
        OTHER_USES_HELD.swap(0, Ordering::Relaxed)
    ));
}

fn hook(
    calls: GameCalls,
    address: GameAddress,
    name: &str,
    detour: *mut c_void,
    original: &'static AtomicPtr<c_void>,
) -> bool {
    #[expect(
        unsafe_code,
        reason = "hooks a game function; the replacement has the signature of the original"
    )]
    // SAFETY: the address is a function of the verified build whose start bytes were just
    // checked, and the replacement takes and returns what the original does.
    unsafe {
        nc_hook::detour::hook_address(
            name,
            calls.address_value(address) as usize,
            detour,
            original,
        )
    }
}

extern "system" fn use_item_detour(inventory: usize, item_id: i32, scale: f32) -> u64 {
    if !item_use_is_held(HELD_HERO.load(Ordering::Relaxed) != 0, item_id) {
        return call_original_use_item(inventory, item_id, scale);
    }
    let (asker, return_rva) = HOLD_CONTEXT
        .get()
        .map_or((ItemUseAsker::Unknown, 0), |context| {
            find_the_asker(context)
        });
    OTHER_USES_HELD.fetch_add(1, Ordering::Relaxed);
    log_held_use(&format!(
        "downed: an item use was held back (item id {item_id:#X}, asked by {} at return {return_rva:#X}); the downed hero's item count stays as it is",
        asker.label()
    ));
    NOT_USED
}

extern "system" fn auto_item_use_check_detour(body: usize) {
    let held_hero = HELD_HERO.load(Ordering::Relaxed);
    let on_the_held_partner_body = guarded_held_body().is_some_and(|(_, held)| held == body);
    let on_the_held_hero = held_hero != 0 && held_hero == body;
    if !auto_item_use_check_is_skipped(on_the_held_partner_body, on_the_held_hero) {
        return call_original_auto_item_use_check(body);
    }
    if on_the_held_partner_body {
        PARTNER_BODY_CHECKS_SKIPPED.fetch_add(1, Ordering::Relaxed);
        if !FIRST_PARTNER_BODY_CHECK_SKIP_LOGGED.swap(true, Ordering::Relaxed) {
            log_line(
                "partner: the Auto-Use Item chip check skipped on the partner body (its health is the other player's; it never spends this game's items or heals this hero; skipped at all times, logged once)",
            );
        }
        return;
    }
    let Some(context) = HOLD_CONTEXT.get() else {
        return;
    };
    let Some(reading) = read_auto_item_use(context, body) else {
        return;
    };
    let would_use = AUTO_CHECK_HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .held_check_would_use(reading, context.health_share);
    if would_use {
        AUTOMATIC_USES_HELD.fetch_add(1, Ordering::Relaxed);
        log_held_use(&format!(
            "downed: an item use was held back (asked by {} at level {}, health {} of {} under its share {}, cool-down over; an item is used there only when one is in stock); the downed hero's item count stays as it is",
            ItemUseAsker::AutoUseItemChip.label(),
            reading.chip_level,
            reading.health,
            reading.max_health,
            context.health_share
        ));
    }
}

fn read_auto_item_use(context: &HoldContext, body: usize) -> Option<AutoItemUseReading> {
    if !context.chip_level_is_trusted {
        return None;
    }
    let calls = context.calls;
    let float = |offset| calls.read_f32_at(calls.body_field(body, offset)).ok();
    Some(AutoItemUseReading {
        chip_level: chip_level(calls, body)?,
        health: calls
            .read_i32_at(calls.body_field(body, BODY_HEALTH_OFFSET))
            .ok()?,
        max_health: calls.read_max_health(body)?,
        game_cool_down: float(BODY_AUTO_ITEM_USE_COOL_DOWN_OFFSET)?,
        step: float(BODY_STEP_OFFSET)?,
    })
}

#[expect(
    unsafe_code,
    reason = "calls the game's chip level function, which only reads the body's fitted chips"
)]
fn chip_level(calls: GameCalls, body: usize) -> Option<i32> {
    let function = calls.address_value(CHIP_LEVEL_FUNCTION) as usize;
    // SAFETY: the function's start bytes were checked at install; it reads the chip list of the
    // body the game itself just passed to its chip check, on the game thread that called it.
    Some(unsafe {
        core::mem::transmute::<usize, ChipLevel>(function)(body, AUTO_ITEM_USE_CHIP_TYPE)
    })
}

#[expect(
    unsafe_code,
    reason = "walks the stack to find the game code that called the use-item function"
)]
fn find_the_asker(context: &HoldContext) -> (ItemUseAsker, u32) {
    let image_base = context
        .calls
        .address_value(USE_ITEM_FUNCTION)
        .wrapping_sub(u64::from(USE_ITEM_FUNCTION.rva(context.variant)));
    let mut frames = [ptr::null_mut::<c_void>(); CALLER_SEARCH_FRAMES];
    // SAFETY: the buffer holds the frames asked for; the hash out-pointer may be null.
    let captured = unsafe {
        RtlCaptureStackBackTrace(
            0,
            CALLER_SEARCH_FRAMES as u32,
            frames.as_mut_ptr(),
            ptr::null_mut(),
        )
    };
    let game_returns: Vec<u32> = frames
        .iter()
        .take(usize::from(captured))
        .map(|frame| (*frame as usize as u64).wrapping_sub(image_base))
        .filter(|rva| *rva < GAME_IMAGE_RVA_LIMIT)
        .filter_map(|rva| u32::try_from(rva).ok())
        .collect();
    game_returns
        .iter()
        .map(|rva| (item_use_asker(context.variant, *rva), *rva))
        .find(|(asker, _)| *asker != ItemUseAsker::Unknown)
        .or_else(|| {
            game_returns
                .first()
                .map(|rva| (ItemUseAsker::Unknown, *rva))
        })
        .unwrap_or((ItemUseAsker::Unknown, 0))
}

fn log_held_use(line: &str) {
    if HELD_USE_LINES_LOGGED.fetch_add(1, Ordering::Relaxed) < ITEM_USES_HELD_LOGGED_AT_MOST {
        log_line(line);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original use-item function through the trampoline MinHook built"
)]
fn call_original_use_item(inventory: usize, item_id: i32, scale: f32) -> u64 {
    let original = ORIGINAL_USE_ITEM.load(Ordering::Acquire);
    if original.is_null() {
        return NOT_USED;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    unsafe { core::mem::transmute::<*mut c_void, UseItem>(original)(inventory, item_id, scale) }
}

#[expect(
    unsafe_code,
    reason = "calls the original chip check through the trampoline MinHook built"
)]
fn call_original_auto_item_use_check(body: usize) {
    let original = ORIGINAL_AUTO_ITEM_USE_CHECK.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function with the body received.
    unsafe { core::mem::transmute::<*mut c_void, AutoItemUseCheck>(original)(body) }
}
