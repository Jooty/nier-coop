use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};

use replicant_game::GameMemory;
use replicant_game::script_functions::{
    DEBUG_PRINT_NATIVE_RVA, debug_print_tag_range, debug_print_text_range, traced_function_name,
    traced_function_text_from_parts,
};

use crate::build_gate::VerifiedBuild;
use crate::cutscene_skip::on_function_traced;
use crate::game_hooks::{hook_address, note_native};
use crate::memory::InProcessMemory;
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
    reason = "calls the original native through the trampoline"
)]
extern "system" fn debug_print_detour(args: *mut c_void) {
    let args_address = args as u64;
    let _ = panic::catch_unwind(|| {
        note_native("_DebugPrint");
        note_printed_text(args_address);
    });
    let trampoline = ORIGINAL_DEBUG_PRINT.load(Ordering::Acquire);
    if !trampoline.is_null() {
        // SAFETY: the trampoline continues into the original native, which has this signature.
        unsafe { core::mem::transmute::<*mut c_void, LuaNative>(trampoline)(args) };
    }
}

fn note_printed_text(args_address: u64) {
    if args_address == 0 {
        return;
    }
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    let tag_range = debug_print_tag_range();
    let mut tag = vec![0_u8; tag_range.len()];
    if memory
        .read(args_address + tag_range.start as u64, &mut tag)
        .is_err()
    {
        return;
    }
    // the text is read only after the tag says this call really has a text argument
    let text_range = debug_print_text_range();
    let mut text = vec![0_u8; text_range.len()];
    if memory
        .read(args_address + text_range.start as u64, &mut text)
        .is_err()
    {
        return;
    }
    let Some(text) = traced_function_text_from_parts(&tag, &text) else {
        return;
    };
    if let Some(name) = traced_function_name(&text) {
        on_function_traced(build, name);
    }
}
