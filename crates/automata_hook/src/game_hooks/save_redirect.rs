use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

use automata_game::address::BUILD_SAVE_FILE_PATH_FUNCTION;
use automata_game::build::variant_of_image_size;
use automata_game::save_redirect::{
    SAVE_PATH_BUFFER_LEN, SaveRedirect, SaveRedirectStatus, SaveRole, redirect_save_path,
};
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;

static ORIGINAL_BUILD_SAVE_FILE_PATH: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SAVE_ROLE: OnceLock<SaveRole> = OnceLock::new();
static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static COOP_FOLDER: OnceLock<String> = OnceLock::new();
static FOLDER_LOGGED: AtomicBool = AtomicBool::new(false);
static FAILURE_LOGGED: AtomicBool = AtomicBool::new(false);

type BuildSaveFilePath = unsafe extern "system" fn(usize, *mut u8) -> u64;

pub fn save_redirect_status() -> SaveRedirectStatus {
    if HOOK_INSTALLED.load(Ordering::Acquire) {
        SaveRedirectStatus::CoopFolder
    } else {
        SaveRedirectStatus::NotRedirected
    }
}

pub fn coop_save_folder() -> Option<String> {
    COOP_FOLDER.get().cloned()
}

pub fn install_save_redirect(build: VerifiedBuild, role: SaveRole) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line(SaveRedirectStatus::NotRedirected.status_line());
        return;
    };
    if SAVE_ROLE.set(role).is_err() {
        return;
    }
    let address =
        build.module_base() as usize + BUILD_SAVE_FILE_PATH_FUNCTION.rva(variant) as usize;
    #[expect(
        unsafe_code,
        reason = "hooks the game's save file path builder; the replacement forwards both arguments"
    )]
    // SAFETY: the address is Hw::BuildSaveFilePath of the verified build (a guess until run); the
    // replacement has its signature (device pointer, output buffer) and returns the original's rax.
    let hooked = unsafe {
        nc_hook::detour::hook_address(
            "save file path builder (co-op save folder)",
            address,
            build_save_file_path_detour as *const () as *mut c_void,
            &ORIGINAL_BUILD_SAVE_FILE_PATH,
        )
    };
    if hooked {
        HOOK_INSTALLED.store(true, Ordering::Release);
    } else {
        log_line(SaveRedirectStatus::NotRedirected.status_line());
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original path builder through the trampoline MinHook built"
)]
extern "system" fn build_save_file_path_detour(device: usize, out: *mut u8) -> u64 {
    let original = ORIGINAL_BUILD_SAVE_FILE_PATH.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    let result =
        unsafe { core::mem::transmute::<*mut c_void, BuildSaveFilePath>(original)(device, out) };
    if result as u32 == 0 || out.is_null() {
        return result;
    }
    match panic::catch_unwind(|| redirect_output_buffer(out)) {
        Ok(true) => result,
        _ => {
            log_failure_once("saves: could not redirect a save path, the game gets a failed path");
            0
        }
    }
}

#[expect(
    unsafe_code,
    reason = "rewrites the game's path buffer, which holds SAVE_PATH_BUFFER_LEN bytes"
)]
fn redirect_output_buffer(out: *mut u8) -> bool {
    // SAFETY: the game passes a buffer of SAVE_PATH_BUFFER_LEN bytes and the builder just filled it.
    let buffer = unsafe { core::slice::from_raw_parts_mut(out, SAVE_PATH_BUFFER_LEN) };
    let Some(text_len) = buffer.iter().position(|byte| *byte == 0) else {
        return false;
    };
    let Ok(path) = core::str::from_utf8(&buffer[..text_len]) else {
        return false;
    };
    let role = SAVE_ROLE.get().copied().unwrap_or(SaveRole::Host);
    match redirect_save_path(path, role) {
        SaveRedirect::AlreadyRedirected => true,
        SaveRedirect::NotASavePath | SaveRedirect::TooLong => false,
        SaveRedirect::Redirected {
            path: redirected,
            folder,
        } => {
            if let Err(error) = std::fs::create_dir_all(&folder) {
                log_failure_once(&format!("saves: cannot create {folder} ({error})"));
            }
            let _ = COOP_FOLDER.set(folder.clone());
            if !FOLDER_LOGGED.swap(true, Ordering::Relaxed) {
                log_line(&format!("saves: co-op folder {folder}"));
            }
            buffer[..redirected.len()].copy_from_slice(redirected.as_bytes());
            buffer[redirected.len()] = 0;
            true
        }
    }
}

fn log_failure_once(text: &str) {
    if !FAILURE_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(text);
    }
}
