use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use automata_game::build::variant_of_image_size;
use automata_game::exe::ExeVariant;
use automata_game::input_block::{InputHook, input_block_plan, prologue_matches};
use nt_hook::build_gate::VerifiedBuild;
use nt_hook::diagnostics::log_line;
use nt_hook::memory::{GameMemory, InProcessMemory};

const LONGEST_PROLOGUE_LEN: usize = 13;

static MENU_OPEN: AtomicBool = AtomicBool::new(false);
static ORIGINAL_POLL_DEVICES: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_POLL_MOUSE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_CURSOR_RECENTRE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static BLOCKED_BUILD: OnceLock<(VerifiedBuild, ExeVariant)> = OnceLock::new();
static WRITE_FAILURE_LOGGED: AtomicBool = AtomicBool::new(false);

type PollDevices = unsafe extern "system" fn();
type PollMouse = unsafe extern "system" fn(usize, usize, usize) -> usize;
type CursorRecentre = unsafe extern "system" fn(f32, f32);

pub fn set_menu_open(open: bool) {
    MENU_OPEN.store(open, Ordering::Relaxed);
}

pub fn install_input_block(build: VerifiedBuild) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("input block: image size matches no known exe, not installed");
        return;
    };
    if BLOCKED_BUILD.set((build, variant)).is_err() {
        return;
    }
    for hook in InputHook::ALL {
        install_one(build, variant, hook);
    }
}

fn install_one(build: VerifiedBuild, variant: ExeVariant, hook: InputHook) {
    let address = build.module_base() as usize + hook.address().rva(variant) as usize;
    let found = InProcessMemory::new(&build).read_array::<LONGEST_PROLOGUE_LEN>(address as u64);
    if !found.is_ok_and(|bytes| prologue_matches(hook, &bytes)) {
        log_line(&format!(
            "input block: the first bytes of the {} are not the noted ones, not hooked",
            hook.label()
        ));
        return;
    }
    let (replacement, original) = match hook {
        InputHook::PollDevices => (poll_devices_detour as *const (), &ORIGINAL_POLL_DEVICES),
        InputHook::PollMouse => (poll_mouse_detour as *const (), &ORIGINAL_POLL_MOUSE),
        InputHook::CursorRecentre => (
            cursor_recentre_detour as *const (),
            &ORIGINAL_CURSOR_RECENTRE,
        ),
    };
    #[expect(
        unsafe_code,
        reason = "hooks a game input function; each replacement has the signature of its original"
    )]
    // SAFETY: the address passed the first-bytes check for this function of the verified build (a
    // guess for the signature until run) and the replacement forwards to the original.
    unsafe {
        nt_hook::detour::hook_address(hook.label(), address, replacement as *mut c_void, original);
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original device poll through the trampoline MinHook built"
)]
extern "system" fn poll_devices_detour() {
    let original = ORIGINAL_POLL_DEVICES.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function, which takes no arguments.
    unsafe { core::mem::transmute::<*mut c_void, PollDevices>(original)() };
    zero_sources_while_menu_open();
}

#[expect(
    unsafe_code,
    reason = "calls the original mouse poll through the trampoline MinHook built"
)]
extern "system" fn poll_mouse_detour(first: usize, out: usize, active: usize) -> usize {
    let original = ORIGINAL_POLL_MOUSE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let active_given = current_plan()
        .and_then(|plan| plan.mouse_active_override)
        .map_or(active, |value| value as u32 as usize);
    // SAFETY: the trampoline continues into the original with the arguments received, except the
    // third, which the menu may override.
    unsafe { core::mem::transmute::<*mut c_void, PollMouse>(original)(first, out, active_given) }
}

#[expect(
    unsafe_code,
    reason = "calls the original cursor recentre through the trampoline MinHook built"
)]
extern "system" fn cursor_recentre_detour(x: f32, y: f32) {
    if current_plan().is_some_and(|plan| plan.skip_cursor_recentre) {
        return;
    }
    let original = ORIGINAL_CURSOR_RECENTRE.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original with the two float arguments received.
    unsafe { core::mem::transmute::<*mut c_void, CursorRecentre>(original)(x, y) };
}

fn current_plan() -> Option<automata_game::input_block::InputBlockPlan> {
    let (_, variant) = BLOCKED_BUILD.get()?;
    Some(input_block_plan(
        MENU_OPEN.load(Ordering::Relaxed),
        *variant,
    ))
}

fn zero_sources_while_menu_open() {
    let (Some(plan), Some((build, _))) = (current_plan(), BLOCKED_BUILD.get()) else {
        return;
    };
    let mut memory = InProcessMemory::new(build);
    for span in plan.zero_after_poll_devices {
        let zeros = vec![0_u8; span.len as usize];
        let address = build.module_base() + u64::from(span.rva);
        if memory.write(address, &zeros).is_err()
            && !WRITE_FAILURE_LOGGED.swap(true, Ordering::Relaxed)
        {
            log_line("input block: cannot zero an input source");
        }
    }
}
