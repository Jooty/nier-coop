use core::ffi::c_void;
use core::ptr;
use std::panic;

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::CreateThread;

use crate::diagnostics::log_line;
use crate::overlay::install_when_renderer_ready;

pub(super) fn spawn_init_thread() {
    #[expect(
        unsafe_code,
        reason = "the loader lock forbids real work in DllMain, so a thread is started through the Win32 API"
    )]
    // SAFETY: the start routine ignores its parameter and a null thread id is allowed.
    let thread = unsafe {
        CreateThread(
            ptr::null(),
            0,
            Some(init_thread_main),
            ptr::null(),
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

extern "system" fn init_thread_main(_parameter: *mut c_void) -> u32 {
    if panic::catch_unwind(install_when_renderer_ready).is_err() {
        log_line("init thread panicked");
    }
    0
}
