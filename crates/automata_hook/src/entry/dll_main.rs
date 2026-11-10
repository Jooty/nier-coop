use core::ffi::c_void;

use nc_hook::diagnostics::{LogSettings, configure};
use nc_hook::entry::handle_dll_event;
use windows_sys::Win32::Foundation::{HINSTANCE, TRUE};
use windows_sys::Win32::System::SystemServices::DLL_PROCESS_ATTACH;
use windows_sys::core::BOOL;

use crate::BUILD_COMMIT;
use crate::game_hooks::install_second_copy_guard;
use crate::log_settings::{LOG_ENV, log_from_config_beside_game, resolve_log_path};
use crate::overlay::install_when_renderer_ready;

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
    if reason == DLL_PROCESS_ATTACH {
        install_second_copy_guard();
    }
    handle_dll_event(reason, reserved, install_when_renderer_ready);
    TRUE
}

fn automata_log_path() -> String {
    resolve_log_path(std::env::var(LOG_ENV).ok(), log_from_config_beside_game)
}
