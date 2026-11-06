use core::ffi::c_void;

use windows_sys::Win32::Foundation::{BOOL, HINSTANCE, TRUE};

#[expect(unsafe_code, reason = "the Windows loader calls DllMain by its exported name")]
#[unsafe(no_mangle)]
pub extern "system" fn DllMain(_module: HINSTANCE, _reason: u32, _reserved: *mut c_void) -> BOOL {
    TRUE
}
