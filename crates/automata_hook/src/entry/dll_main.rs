use core::ffi::c_void;

use nc_hook::diagnostics::{LogSettings, configure};
use nc_hook::entry::handle_dll_event;
use windows_sys::Win32::Foundation::{HINSTANCE, TRUE};
use windows_sys::core::BOOL;

use crate::boot::run_gate;
use crate::log_settings::{LOG_ENV, resolve_log_path};

const BUILD_COMMIT: &str = env!("NC_BUILD_COMMIT");

#[expect(
    unsafe_code,
    reason = "the Windows loader calls DllMain by its exported name"
)]
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HINSTANCE, reason: u32, reserved: *mut c_void) -> BOOL {
    configure(LogSettings {
        build_commit: BUILD_COMMIT,
        resolve_path: automata_log_path,
    });
    handle_dll_event(reason, reserved, run_gate);
    TRUE
}

fn automata_log_path() -> String {
    resolve_log_path(std::env::var(LOG_ENV).ok())
}
