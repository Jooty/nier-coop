use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::game_hooks::hook_slot::hook_user32_export;
use crate::harness;
use crate::overlay;

static ORIGINAL_GET_KEY_STATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_GET_ASYNC_KEY_STATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_GET_FOREGROUND_WINDOW: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_GET_ACTIVE_WINDOW: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_GET_FOCUS: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type KeyStateFn = unsafe extern "system" fn(i32) -> i16;
type WindowFn = unsafe extern "system" fn() -> *mut c_void;

pub(super) fn install() {
    hook_user32_export(
        windows_sys::s!("GetKeyState"),
        "user32 GetKeyState",
        get_key_state_detour as *const () as *mut c_void,
        &ORIGINAL_GET_KEY_STATE,
    );
    hook_user32_export(
        windows_sys::s!("GetAsyncKeyState"),
        "user32 GetAsyncKeyState",
        get_async_key_state_detour as *const () as *mut c_void,
        &ORIGINAL_GET_ASYNC_KEY_STATE,
    );
    hook_user32_export(
        windows_sys::s!("GetForegroundWindow"),
        "user32 GetForegroundWindow",
        get_foreground_window_detour as *const () as *mut c_void,
        &ORIGINAL_GET_FOREGROUND_WINDOW,
    );
    hook_user32_export(
        windows_sys::s!("GetActiveWindow"),
        "user32 GetActiveWindow",
        get_active_window_detour as *const () as *mut c_void,
        &ORIGINAL_GET_ACTIVE_WINDOW,
    );
    hook_user32_export(
        windows_sys::s!("GetFocus"),
        "user32 GetFocus",
        get_focus_detour as *const () as *mut c_void,
        &ORIGINAL_GET_FOCUS,
    );
}

#[expect(
    unsafe_code,
    reason = "calls the original key state function through the trampoline MinHook built"
)]
fn key_state_with_injection(original: &AtomicPtr<c_void>, virtual_key: i32) -> i16 {
    let original = original.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original, which has this exact signature.
    let state = unsafe { core::mem::transmute::<*mut c_void, KeyStateFn>(original)(virtual_key) };
    let injected = u8::try_from(virtual_key)
        .ok()
        .is_some_and(harness::virtual_key_is_down);
    if injected { state | i16::MIN } else { state }
}

#[expect(
    unsafe_code,
    reason = "calls the original window query through the trampoline MinHook built"
)]
fn game_window_when_known(original: &AtomicPtr<c_void>) -> *mut c_void {
    let original = original.load(Ordering::Acquire);
    // SAFETY: the trampoline continues into the original, which has this exact signature.
    let real = if original.is_null() {
        ptr::null_mut()
    } else {
        unsafe { core::mem::transmute::<*mut c_void, WindowFn>(original)() }
    };
    match overlay::game_window() {
        0 => real,
        window => window as *mut c_void,
    }
}

extern "system" fn get_key_state_detour(virtual_key: i32) -> i16 {
    key_state_with_injection(&ORIGINAL_GET_KEY_STATE, virtual_key)
}

extern "system" fn get_async_key_state_detour(virtual_key: i32) -> i16 {
    key_state_with_injection(&ORIGINAL_GET_ASYNC_KEY_STATE, virtual_key)
}

extern "system" fn get_foreground_window_detour() -> *mut c_void {
    game_window_when_known(&ORIGINAL_GET_FOREGROUND_WINDOW)
}

extern "system" fn get_active_window_detour() -> *mut c_void {
    game_window_when_known(&ORIGINAL_GET_ACTIVE_WINDOW)
}

extern "system" fn get_focus_detour() -> *mut c_void {
    game_window_when_known(&ORIGINAL_GET_FOCUS)
}
