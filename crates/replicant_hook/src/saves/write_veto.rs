use core::ffi::{CStr, c_char, c_void};
use core::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

use replicant_game::offsets::SAVE_IO_WRITE_FILE_RVA;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;

const WRITE_SUCCEEDED: u8 = 0;

static ORIGINAL_WRITE_FILE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static VETOED_WRITES: AtomicU64 = AtomicU64::new(0);

pub fn install(build: VerifiedBuild) {
    let address = (build.module_base() + SAVE_IO_WRITE_FILE_RVA) as usize;
    #[expect(unsafe_code, reason = "hooks SaveIo::WriteFile by its address")]
    // SAFETY: the RVA starts SaveIo::WriteFile (io, name, data, size) -> char in the verified
    // build, and the replacement has the same signature.
    unsafe {
        hook_address(
            "SaveIo::WriteFile",
            address,
            write_file_detour as *const () as *mut c_void,
            &ORIGINAL_WRITE_FILE,
        )
    };
}

#[expect(unsafe_code, reason = "reads the file name the game passed in")]
extern "system" fn write_file_detour(
    _io: *mut c_void,
    name: *const c_char,
    _data: *const c_void,
    size: u32,
) -> u8 {
    let name = if name.is_null() {
        "<null>".to_owned()
    } else {
        // SAFETY: the game passes a NUL-terminated file name.
        unsafe { CStr::from_ptr(name) }
            .to_string_lossy()
            .into_owned()
    };
    let count = VETOED_WRITES.fetch_add(1, Ordering::Relaxed) + 1;
    log_line(&format!(
        "saves: vetoed write #{count} ({name}, {size} bytes)"
    ));
    WRITE_SUCCEEDED
}
