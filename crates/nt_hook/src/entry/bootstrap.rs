use core::ffi::c_void;
use core::ptr;
use std::panic;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::CreateThread;

use crate::diagnostics::log_line;

pub(super) fn spawn_init_thread(init: fn()) {
    #[expect(
        unsafe_code,
        reason = "the loader lock forbids real work in DllMain, so a thread is started through the Win32 API"
    )]
    // SAFETY: the start routine only turns the parameter back into the `fn()` passed here, and a
    // null thread id is allowed.
    let thread = unsafe {
        CreateThread(
            ptr::null(),
            0,
            Some(init_thread_main),
            init as *const () as *const c_void,
            0,
            ptr::null_mut(),
        )
    };
    if !thread.is_null() {
        #[expect(unsafe_code, reason = "closing the handle of the thread just started")]
        // SAFETY: `thread` is a valid handle returned by CreateThread and is closed once.
        unsafe {
            CloseHandle(thread)
        };
    }
}

extern "system" fn init_thread_main(parameter: *mut c_void) -> u32 {
    #[expect(
        unsafe_code,
        reason = "turns the thread parameter back into the init function"
    )]
    // SAFETY: `spawn_init_thread` passed a `fn()` as the parameter and nothing else starts this routine.
    let init = unsafe { core::mem::transmute::<*mut c_void, fn()>(parameter) };
    crate::crash_log::install();
    if panic::catch_unwind(init).is_err() {
        log_line("init thread panicked");
    }
    0
}
