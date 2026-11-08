use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::script_functions::{
    DEBUG_PRINT_NATIVE_RVA, debug_print_args_len, traced_function_name, traced_function_text,
};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::on_function_traced;
use crate::game_hooks::hook_address;
use crate::story_relay::state::HOOKED_BUILD;

static ORIGINAL_DEBUG_PRINT: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type LuaNative = unsafe extern "system" fn(*mut c_void);

pub(super) fn install(build: VerifiedBuild) {
    #[expect(
        unsafe_code,
        reason = "hooks the shared no-op script native that _DebugPrint is bound to, to read the function names scripts print"
    )]
    // SAFETY: the RVA starts the 3-byte no-op native (ret) that the binding table (0x1306710) gives
    // _DebugPrint and 29 other natives; the detour has the same signature and calls the original.
    unsafe {
        hook_address(
            "LuaNative::DebugPrint",
            (build.module_base() + u64::from(DEBUG_PRINT_NATIVE_RVA)) as usize,
            debug_print_detour as *const () as *mut c_void,
            &ORIGINAL_DEBUG_PRINT,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "copies the native's argument block and calls the original through the trampoline"
)]
extern "system" fn debug_print_detour(args: *mut c_void) {
    let args_address = args as u64;
    let _ = panic::catch_unwind(|| note_printed_text(args_address));
    let trampoline = ORIGINAL_DEBUG_PRINT.load(Ordering::Acquire);
    if !trampoline.is_null() {
        // SAFETY: the trampoline continues into the original native, which has this signature.
        unsafe { core::mem::transmute::<*mut c_void, LuaNative>(trampoline)(args) };
    }
}

#[expect(
    unsafe_code,
    reason = "reads the first argument slots of the native's argument block, which it may read itself"
)]
fn note_printed_text(args_address: u64) {
    if args_address == 0 {
        return;
    }
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let mut block = vec![0_u8; debug_print_args_len()];
    // SAFETY: every native receives the Lua argument block, which holds at least the header and
    // the first argument slot (text capacity included) these bytes cover.
    unsafe {
        ptr::copy_nonoverlapping(args_address as *const u8, block.as_mut_ptr(), block.len());
    }
    let Some(text) = traced_function_text(&block) else {
        return;
    };
    if let Some(name) = traced_function_name(&text) {
        on_function_traced(build, name);
    }
}
