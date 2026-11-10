use core::ffi::c_void;
use core::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use crate::detour::Detour;
use crate::diagnostics::log_line;

/// # Safety
/// `address` must start a function in this process; `replacement` must match its signature.
#[expect(
    unsafe_code,
    reason = "hooking a function by address is inherently unchecked"
)]
pub unsafe fn hook_address(
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
