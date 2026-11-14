use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

use replicant_game::offsets::TEXT_TABLE_GETTER_FUNCTION_RVA;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;

static ORIGINAL_GETTER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static OFFERED_TO_THREAD: AtomicU32 = AtomicU32::new(0);
static OFFERED_TEXT: AtomicPtr<c_char> = AtomicPtr::new(ptr::null_mut());

type GetText = unsafe extern "system" fn(*mut c_void, i32) -> *const c_char;

pub(crate) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    let address = (build.module_base() + u64::from(TEXT_TABLE_GETTER_FUNCTION_RVA)) as usize;
    #[expect(unsafe_code, reason = "hooks the text table getter by address")]
    // SAFETY: the RVA starts CTextDataMan::GetText (table pointer, int id; returns the text) in
    // the verified build, and the replacement has the same signature.
    unsafe {
        hook_address(
            "CTextDataMan_GetText",
            address,
            getter_detour as *const () as *mut c_void,
            &ORIGINAL_GETTER,
        )
    };
}

pub(super) fn is_installed() -> bool {
    !ORIGINAL_GETTER.load(Ordering::Acquire).is_null()
}

pub(super) fn offer(text: &str) -> bool {
    let Ok(text) = CString::new(text) else {
        return false;
    };
    let fresh = text.into_raw();
    let previous = OFFERED_TEXT.swap(fresh, Ordering::AcqRel);
    free_offered(previous);
    #[expect(unsafe_code, reason = "reads the calling thread's id")]
    // SAFETY: GetCurrentThreadId takes no arguments and cannot fail.
    let thread = unsafe { GetCurrentThreadId() };
    OFFERED_TO_THREAD.store(thread, Ordering::Release);
    true
}

pub(super) fn withdraw() {
    OFFERED_TO_THREAD.store(0, Ordering::Release);
}

fn free_offered(text: *mut c_char) {
    if text.is_null() {
        return;
    }
    #[expect(unsafe_code, reason = "frees a string this module made with into_raw")]
    // SAFETY: every non-null pointer in OFFERED_TEXT came from CString::into_raw here, and only
    // the picker's thread, one call at a time, replaces it after the game has copied the text.
    drop(unsafe { CString::from_raw(text) });
}

#[expect(
    unsafe_code,
    reason = "calls the original getter through the trampoline MinHook built"
)]
extern "system" fn getter_detour(table: *mut c_void, id: i32) -> *const c_char {
    let original = ORIGINAL_GETTER.load(Ordering::Acquire);
    if original.is_null() {
        return ptr::null();
    }
    let offered_to = OFFERED_TO_THREAD.load(Ordering::Acquire);
    // SAFETY: GetCurrentThreadId takes no arguments and cannot fail.
    if offered_to != 0 && offered_to == unsafe { GetCurrentThreadId() } {
        OFFERED_TO_THREAD.store(0, Ordering::Release);
        let offered = OFFERED_TEXT.load(Ordering::Acquire);
        if !offered.is_null() {
            return offered;
        }
    }
    // SAFETY: the trampoline continues into the original getter, which has this exact signature.
    unsafe { core::mem::transmute::<*mut c_void, GetText>(original)(table, id) }
}
