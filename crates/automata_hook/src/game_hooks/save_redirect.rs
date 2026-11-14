use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::time::SystemTime;

use automata_game::address::BUILD_SAVE_FILE_PATH_FUNCTION;
use automata_game::build::variant_of_image_size;
use automata_game::save_redirect::{
    SAVE_PATH_BUFFER_LEN, SaveRedirect, SaveRedirectStatus, SaveRole, coop_folder_levels,
    redirect_save_path,
};
use automata_game::start::HIGHEST_SAVE_SLOT;
use nt_hook::build_gate::VerifiedBuild;
use nt_hook::diagnostics::log_line;
use windows_sys::Win32::Storage::FileSystem::{
    CreateDirectoryA, FILE_ATTRIBUTE_DIRECTORY, GetFileAttributesA, INVALID_FILE_ATTRIBUTES,
};

static ORIGINAL_BUILD_SAVE_FILE_PATH: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static SAVE_ROLE: OnceLock<SaveRole> = OnceLock::new();
static HOOK_INSTALLED: AtomicBool = AtomicBool::new(false);
static COOP_FOLDER: OnceLock<Vec<u8>> = OnceLock::new();
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

pub fn coop_save_file_exists(file_name: &str) -> bool {
    let Some(folder) = COOP_FOLDER.get() else {
        return false;
    };
    let mut path = folder.clone();
    path.push(b'\\');
    path.extend_from_slice(file_name.as_bytes());
    attributes_of(path).is_some_and(|attributes| attributes & FILE_ATTRIBUTE_DIRECTORY == 0)
}

pub fn coop_save_slot_times() -> Vec<(u32, SystemTime)> {
    let Some(folder) = COOP_FOLDER.get() else {
        return Vec::new();
    };
    let folder = String::from_utf8_lossy(folder).into_owned();
    (0..=HIGHEST_SAVE_SLOT)
        .filter_map(|slot| {
            let path = format!("{folder}\\SlotData_{slot}.dat");
            let written = std::fs::metadata(path)
                .ok()
                .filter(std::fs::Metadata::is_file)?
                .modified()
                .ok()?;
            Some((slot, written))
        })
        .collect()
}

pub fn install_save_redirect(build: VerifiedBuild, role: SaveRole) -> bool {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        return false;
    };
    if SAVE_ROLE.set(role).is_err() {
        return HOOK_INSTALLED.load(Ordering::Acquire);
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
        nt_hook::detour::hook_address(
            "save file path builder (co-op save folder)",
            address,
            build_save_file_path_detour as *const () as *mut c_void,
            &ORIGINAL_BUILD_SAVE_FILE_PATH,
        )
    };
    if hooked {
        HOOK_INSTALLED.store(true, Ordering::Release);
        log_line("saves: display settings stay in the normal folder");
    }
    hooked
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
    let role = SAVE_ROLE.get().copied().unwrap_or(SaveRole::Host);
    match redirect_save_path(&buffer[..text_len], role) {
        SaveRedirect::AlreadyRedirected => true,
        SaveRedirect::NotASavePath | SaveRedirect::TooLong => false,
        SaveRedirect::Redirected {
            path: redirected,
            folder,
        } => {
            let folder_text = String::from_utf8_lossy(&folder).into_owned();
            if !create_folder(&folder) {
                log_failure_once(&format!("saves: cannot create {folder_text}"));
            }
            if !FOLDER_LOGGED.swap(true, Ordering::Relaxed) {
                log_line(&format!("saves: co-op folder {folder_text}"));
            }
            let _ = COOP_FOLDER.set(folder);
            buffer[..redirected.len()].copy_from_slice(&redirected);
            buffer[redirected.len()] = 0;
            true
        }
    }
}

fn create_folder(folder: &[u8]) -> bool {
    for mut level in coop_folder_levels(folder) {
        level.push(0);
        #[expect(
            unsafe_code,
            reason = "creates one folder level with the ANSI call the game's own paths use"
        )]
        // SAFETY: `level` is a NUL-terminated path that outlives the call; a level that exists
        // already fails harmlessly.
        unsafe {
            CreateDirectoryA(level.as_ptr(), ptr::null());
        }
    }
    attributes_of(folder.to_vec())
        .is_some_and(|attributes| attributes & FILE_ATTRIBUTE_DIRECTORY != 0)
}

fn attributes_of(mut path: Vec<u8>) -> Option<u32> {
    path.push(0);
    #[expect(unsafe_code, reason = "asks Windows for the attributes of a path")]
    // SAFETY: `path` is a NUL-terminated path that outlives the call.
    let attributes = unsafe { GetFileAttributesA(path.as_ptr()) };
    (attributes != INVALID_FILE_ATTRIBUTES).then_some(attributes)
}

fn log_failure_once(text: &str) {
    if !FAILURE_LOGGED.swap(true, Ordering::Relaxed) {
        log_line(text);
    }
}
