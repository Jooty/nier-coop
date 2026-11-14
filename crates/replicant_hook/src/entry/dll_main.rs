use core::ffi::c_void;

use nt_hook::diagnostics::{LogSettings, configure};
use nt_hook::entry::handle_dll_event;
use windows_sys::Win32::Foundation::{HINSTANCE, TRUE};
use windows_sys::core::BOOL;

use crate::net::{AutoConfig, LOG_ENV, resolve_log_path};
use crate::overlay::install_when_renderer_ready;

const BUILD_COMMIT: &str = env!("NT_BUILD_COMMIT");

#[expect(
    unsafe_code,
    reason = "the Windows loader calls DllMain by its exported name"
)]
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HINSTANCE, reason: u32, reserved: *mut c_void) -> BOOL {
    configure(LogSettings {
        build_commit: BUILD_COMMIT,
        resolve_path: replicant_log_path,
    });
    handle_dll_event(reason, reserved, install_when_renderer_ready);
    TRUE
}

fn replicant_log_path() -> String {
    resolve_log_path(std::env::var(LOG_ENV).ok(), || {
        AutoConfig::load_beside_game()
            .ok()
            .flatten()
            .and_then(|config| config.log)
    })
}
