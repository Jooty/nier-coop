use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};

use crate::detour::Detour;
use crate::diagnostics::log_line;

#[expect(
    unsafe_code,
    reason = "hooking a function by address is inherently unchecked"
)]
pub(crate) unsafe fn hook_address(
    label: &str,
    address: usize,
    replacement: *mut c_void,
    original: &AtomicPtr<c_void>,
) -> bool {
    // SAFETY: forwarded from this function's own contract.
    let created = unsafe { Detour::create_at_address(address, replacement) };
    let detour = match created {
        Ok(detour) => detour,
        Err(error) => {
            log_line(&format!("hooks: cannot hook {label} ({error})"));
            return false;
        }
    };
    original.store(detour.trampoline(), Ordering::Release);
    match detour.enable() {
        Ok(()) => {
            log_line(&format!("hooks: {label} hooked"));
            true
        }
        Err(error) => {
            original.store(ptr::null_mut(), Ordering::Release);
            log_line(&format!("hooks: cannot enable {label} ({error})"));
            false
        }
    }
}

#[expect(unsafe_code, reason = "looks up an export of the loaded user32.dll")]
pub(super) fn hook_user32_export(
    export_name: *const u8,
    label: &str,
    replacement: *mut c_void,
    original: &AtomicPtr<c_void>,
) {
    // SAFETY: the module name is a static NUL-terminated string and `export_name` comes from `s!`.
    let address = unsafe {
        let module = GetModuleHandleW(windows_sys::w!("user32.dll"));
        if module.is_null() {
            log_line(&format!("hooks: no {label} (user32.dll is not loaded)"));
            return;
        }
        GetProcAddress(module, export_name)
    };
    let Some(address) = address else {
        log_line(&format!("hooks: no {label} (export not found)"));
        return;
    };
    #[expect(unsafe_code, reason = "hooks a user32 export")]
    // SAFETY: the replacement has the same signature as the export it replaces.
    unsafe {
        hook_address(label, address as usize, replacement, original)
    };
}
