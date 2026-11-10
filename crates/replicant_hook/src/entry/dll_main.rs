use core::ffi::c_void;

use replicant_game::crash_report::detach_line;
use windows_sys::Win32::Foundation::{HINSTANCE, TRUE};
use windows_sys::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::core::BOOL;

use crate::diagnostics::log_line;
use crate::entry::bootstrap::spawn_init_thread;

#[expect(
    unsafe_code,
    reason = "the Windows loader calls DllMain by its exported name"
)]
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HINSTANCE, reason: u32, reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        spawn_init_thread();
    }
    if reason == DLL_PROCESS_DETACH {
        #[expect(unsafe_code, reason = "reads the current thread id")]
        // SAFETY: GetCurrentThreadId takes no arguments and cannot fail.
        let thread_id = unsafe { GetCurrentThreadId() };
        log_line(&detach_line(reserved.is_null(), thread_id));
    }
    TRUE
}
