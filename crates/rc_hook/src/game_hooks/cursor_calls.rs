use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU32, Ordering};

use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows_sys::Win32::UI::WindowsAndMessaging::{ClipCursor, GetCursorPos};

use crate::diagnostics::log_line;
use crate::game_hooks::hook_slot::hook_address;
use crate::game_hooks::mouse_block::cursor_mode_active;

const TRUE_BOOL: i32 = 1;

static ORIGINAL_SET_CURSOR_POS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_CLIP_CURSOR: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_GET_CURSOR_POS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static GAME_LAST_REQUESTED_X: AtomicI32 = AtomicI32::new(0);
static GAME_LAST_REQUESTED_Y: AtomicI32 = AtomicI32::new(0);
static GAME_HAS_REQUESTED_POSITION: AtomicBool = AtomicBool::new(false);
static SPOOFED_GET_CURSOR_POS_CALLS: AtomicU32 = AtomicU32::new(0);
static BLOCKED_SET_CURSOR_POS_CALLS: AtomicU32 = AtomicU32::new(0);
static BLOCKED_CLIP_CURSOR_CALLS: AtomicU32 = AtomicU32::new(0);

type SetCursorPos = unsafe extern "system" fn(i32, i32) -> i32;
type GetCursorPosFn = unsafe extern "system" fn(*mut POINT) -> i32;
type ClipCursorFn = unsafe extern "system" fn(*const c_void) -> i32;

pub fn blocked_set_cursor_pos_calls() -> u32 {
    BLOCKED_SET_CURSOR_POS_CALLS.load(Ordering::Relaxed)
}

pub fn spoofed_get_cursor_pos_calls() -> u32 {
    SPOOFED_GET_CURSOR_POS_CALLS.load(Ordering::Relaxed)
}

#[expect(
    unsafe_code,
    reason = "reads the real cursor through the unhooked GetCursorPos"
)]
pub fn real_cursor_position() -> Option<[i32; 2]> {
    let mut point = POINT { x: 0, y: 0 };
    let original = ORIGINAL_GET_CURSOR_POS.load(Ordering::Acquire);
    // SAFETY: `point` is a valid out pointer, and the trampoline continues into the original GetCursorPos.
    let read = unsafe {
        if original.is_null() {
            GetCursorPos(&raw mut point) != 0
        } else {
            core::mem::transmute::<*mut c_void, GetCursorPosFn>(original)(&raw mut point) != 0
        }
    };
    read.then_some([point.x, point.y])
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
        windows_sys::s!("GetCursorPos"),
        "user32 GetCursorPos",
        get_cursor_pos_detour as *const () as *mut c_void,
        &ORIGINAL_GET_CURSOR_POS,
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
        GAME_LAST_REQUESTED_X.store(x, Ordering::Relaxed);
        GAME_LAST_REQUESTED_Y.store(y, Ordering::Relaxed);
        GAME_HAS_REQUESTED_POSITION.store(true, Ordering::Relaxed);
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

#[expect(
    unsafe_code,
    reason = "calls the original GetCursorPos through the trampoline MinHook built"
)]
extern "system" fn get_cursor_pos_detour(point: *mut POINT) -> i32 {
    if cursor_mode_active()
        && !point.is_null()
        && GAME_HAS_REQUESTED_POSITION.load(Ordering::Relaxed)
    {
        SPOOFED_GET_CURSOR_POS_CALLS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: the game passed a writable POINT.
        unsafe {
            point.write(POINT {
                x: GAME_LAST_REQUESTED_X.load(Ordering::Relaxed),
                y: GAME_LAST_REQUESTED_Y.load(Ordering::Relaxed),
            });
        }
        return TRUE_BOOL;
    }
    let original = ORIGINAL_GET_CURSOR_POS.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, GetCursorPosFn>(original)(point) }
}
