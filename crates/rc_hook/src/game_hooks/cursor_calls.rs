use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::UI::WindowsAndMessaging::ClipCursor;

use crate::diagnostics::log_line;
use crate::game_hooks::hook_slot::hook_address;
use crate::game_hooks::mouse_block::cursor_mode_active;

const TRUE_BOOL: i32 = 1;

static ORIGINAL_SET_CURSOR_POS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_CLIP_CURSOR: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static BLOCKED_SET_CURSOR_POS_CALLS: AtomicU32 = AtomicU32::new(0);
static BLOCKED_CLIP_CURSOR_CALLS: AtomicU32 = AtomicU32::new(0);

type SetCursorPos = unsafe extern "system" fn(i32, i32) -> i32;
type ClipCursorFn = unsafe extern "system" fn(*const c_void) -> i32;

pub fn blocked_set_cursor_pos_calls() -> u32 {
    BLOCKED_SET_CURSOR_POS_CALLS.load(Ordering::Relaxed)
}

pub fn blocked_clip_cursor_calls() -> u32 {
    BLOCKED_CLIP_CURSOR_CALLS.load(Ordering::Relaxed)
}

#[expect(
    unsafe_code,
    reason = "removes any cursor clip through the unhooked ClipCursor"
)]
pub fn clear_cursor_clip() {
    let original = ORIGINAL_CLIP_CURSOR.load(Ordering::Acquire);
    if original.is_null() {
        // SAFETY: a null RECT pointer is the documented way to unclip.
        unsafe { ClipCursor(ptr::null()) };
        return;
    }
    // SAFETY: the trampoline continues into the original ClipCursor, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, ClipCursorFn>(original)(ptr::null()) };
}

pub(super) fn install() {
    hook_user32_export(
        windows_sys::s!("SetCursorPos"),
        "user32 SetCursorPos",
        set_cursor_pos_detour as *const () as *mut c_void,
        &ORIGINAL_SET_CURSOR_POS,
    );
    hook_user32_export(
        windows_sys::s!("ClipCursor"),
        "user32 ClipCursor",
        clip_cursor_detour as *const () as *mut c_void,
        &ORIGINAL_CLIP_CURSOR,
    );
}

#[expect(unsafe_code, reason = "looks up an export of the loaded user32.dll")]
fn hook_user32_export(
    export_name: *const u8,
    label: &str,
    replacement: *mut c_void,
    original: &AtomicPtr<c_void>,
) {
    // SAFETY: the module name is a static NUL-terminated string and `export_name` comes from `s!`.
    let address = unsafe {
        let module = GetModuleHandleW(windows_sys::w!("user32.dll"));
        if module.is_null() {
            log_line(&format!("hooks: no {label} (user32.dll is not loaded)"));
            return;
        }
        GetProcAddress(module, export_name)
    };
    let Some(address) = address else {
        log_line(&format!("hooks: no {label} (export not found)"));
        return;
    };
    #[expect(unsafe_code, reason = "hooks a user32 export")]
    // SAFETY: the replacement has the same signature as the export it replaces.
    unsafe {
        hook_address(label, address as usize, replacement, original)
    };
}

#[expect(
    unsafe_code,
    reason = "calls the original SetCursorPos through the trampoline MinHook built"
)]
extern "system" fn set_cursor_pos_detour(x: i32, y: i32) -> i32 {
    if cursor_mode_active() {
        BLOCKED_SET_CURSOR_POS_CALLS.fetch_add(1, Ordering::Relaxed);
        return TRUE_BOOL;
    }
    let original = ORIGINAL_SET_CURSOR_POS.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, SetCursorPos>(original)(x, y) }
}

#[expect(
    unsafe_code,
    reason = "calls the original ClipCursor through the trampoline MinHook built"
)]
extern "system" fn clip_cursor_detour(rectangle: *const c_void) -> i32 {
    if cursor_mode_active() {
        BLOCKED_CLIP_CURSOR_CALLS.fetch_add(1, Ordering::Relaxed);
        return TRUE_BOOL;
    }
    let original = ORIGINAL_CLIP_CURSOR.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, ClipCursorFn>(original)(rectangle) }
}
