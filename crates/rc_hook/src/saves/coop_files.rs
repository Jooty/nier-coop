use core::ffi::{CStr, c_char, c_void};
use core::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::save_files::coop_save_name;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;

const SAVE_IO_READ_FILE_RVA: u64 = 0x7C5E40;
const SAVE_IO_WRITE_FILE_RVA: u64 = 0x7C6010;

static ORIGINAL_READ_FILE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_WRITE_FILE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type ReadFile = unsafe extern "system" fn(*mut c_void, *const c_char, usize, *mut c_void) -> u64;
type WriteFile = unsafe extern "system" fn(*mut c_void, *const c_char, *const c_void, u32) -> u8;

pub fn install(build: VerifiedBuild, redirect_writes: bool) {
    let read_address = (build.module_base() + SAVE_IO_READ_FILE_RVA) as usize;
    #[expect(unsafe_code, reason = "hooks SaveIo::ReadFile by its address")]
    // SAFETY: the RVA starts SaveIo::ReadFile (io, name, arg, out) -> u64 in the verified build,
    // and the replacement has the same signature.
    unsafe {
        hook_address(
            "SaveIo::ReadFile",
            read_address,
            read_file_detour as *const () as *mut c_void,
            &ORIGINAL_READ_FILE,
        )
    };
    if redirect_writes {
        let write_address = (build.module_base() + SAVE_IO_WRITE_FILE_RVA) as usize;
        #[expect(unsafe_code, reason = "hooks SaveIo::WriteFile by its address")]
        // SAFETY: the RVA starts SaveIo::WriteFile (io, name, data, size) -> char in the verified
        // build, and the replacement has the same signature.
        unsafe {
            hook_address(
                "SaveIo::WriteFile (co-op file)",
                write_address,
                write_file_detour as *const () as *mut c_void,
                &ORIGINAL_WRITE_FILE,
            )
        };
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original ReadFile through the trampoline"
)]
extern "system" fn read_file_detour(
    io: *mut c_void,
    name: *const c_char,
    arg: usize,
    out: *mut c_void,
) -> u64 {
    let original = ORIGINAL_READ_FILE.load(Ordering::Acquire);
    if original.is_null() {
        return 1;
    }
    let name = coop_name_or(name);
    // SAFETY: the trampoline continues into the original ReadFile, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, ReadFile>(original)(io, name, arg, out) }
}

#[expect(
    unsafe_code,
    reason = "calls the original WriteFile through the trampoline"
)]
extern "system" fn write_file_detour(
    io: *mut c_void,
    name: *const c_char,
    data: *const c_void,
    size: u32,
) -> u8 {
    let original = ORIGINAL_WRITE_FILE.load(Ordering::Acquire);
    if original.is_null() {
        return 1;
    }
    let name = coop_name_or(name);
    // SAFETY: the trampoline continues into the original WriteFile, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, WriteFile>(original)(io, name, data, size) }
}

#[expect(unsafe_code, reason = "reads the file name the game passed in")]
fn coop_name_or(name: *const c_char) -> *const c_char {
    if name.is_null() {
        return name;
    }
    // SAFETY: the game passes a NUL-terminated file name.
    let game_name = unsafe { CStr::from_ptr(name) };
    match coop_save_name(game_name.to_bytes()) {
        Some(coop_name) => {
            log_line(&format!("saves: using the co-op file for {game_name:?}"));
            coop_name.as_ptr().cast()
        }
        None => {
            log_line(&format!("saves: {game_name:?} stays as the game named it"));
            name
        }
    }
}
