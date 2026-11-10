use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use automata_game::address::INPUT_UPDATE_ALL_FUNCTION;
use automata_game::build::variant_of_image_size;
use automata_game::exe::ExeVariant;
use automata_game::input_block::spans_to_clear;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::memory::{GameMemory, InProcessMemory};

static MENU_OPEN: AtomicBool = AtomicBool::new(false);
static ORIGINAL_INPUT_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static CLEARED_BUILD: OnceLock<(VerifiedBuild, ExeVariant)> = OnceLock::new();
static WRITE_FAILURE_LOGGED: AtomicBool = AtomicBool::new(false);

type InputUpdate = unsafe extern "system" fn(usize, usize, usize, usize) -> usize;

pub fn set_menu_open(open: bool) {
    MENU_OPEN.store(open, Ordering::Relaxed);
}

pub fn install_input_block(build: VerifiedBuild) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("input block: image size matches no known exe, not installed");
        return;
    };
    if CLEARED_BUILD.set((build, variant)).is_err() {
        return;
    }
    let address = build.module_base() as usize + INPUT_UPDATE_ALL_FUNCTION.rva(variant) as usize;
    #[expect(
        unsafe_code,
        reason = "hooks the game's input update; the replacement passes four register arguments through"
    )]
    // SAFETY: the address is the input update function of the verified build (a guess until run);
    // the replacement forwards its four register arguments and returns the original's rax.
    unsafe {
        nc_hook::detour::hook_address(
            "input update (menu input block)",
            address,
            input_update_detour as *const () as *mut c_void,
            &ORIGINAL_INPUT_UPDATE,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original input update through the trampoline MinHook built"
)]
extern "system" fn input_update_detour(a: usize, b: usize, c: usize, d: usize) -> usize {
    let original = ORIGINAL_INPUT_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    let result = unsafe { core::mem::transmute::<*mut c_void, InputUpdate>(original)(a, b, c, d) };
    if panic::catch_unwind(clear_records_while_menu_open).is_err() {
        log_line("input block: panic while clearing input records");
    }
    result
}

fn clear_records_while_menu_open() {
    let open = MENU_OPEN.load(Ordering::Relaxed);
    let Some((build, variant)) = CLEARED_BUILD.get() else {
        return;
    };
    let mut memory = InProcessMemory::new(build);
    for span in spans_to_clear(open, *variant) {
        let zeros = vec![0_u8; span.len as usize];
        let address = build.module_base() + u64::from(span.rva);
        if memory.write(address, &zeros).is_err()
            && !WRITE_FAILURE_LOGGED.swap(true, Ordering::Relaxed)
        {
            log_line("input block: cannot clear the pad records");
        }
    }
}
