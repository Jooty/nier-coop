use core::ffi::c_void;

use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;

use crate::crash_report::detach_line;
use crate::diagnostics::log_line;
use crate::entry::bootstrap::spawn_init_thread;

pub fn handle_dll_event(reason: u32, reserved: *mut c_void, init: fn()) {
    if reason == DLL_PROCESS_ATTACH {
        spawn_init_thread(init);
    }
    if reason == DLL_PROCESS_DETACH {
        #[expect(unsafe_code, reason = "reads the current thread id")]
        // SAFETY: GetCurrentThreadId takes no arguments and cannot fail.
        let thread_id = unsafe { GetCurrentThreadId() };
        log_line(&detach_line(reserved.is_null(), thread_id));
    }
}
