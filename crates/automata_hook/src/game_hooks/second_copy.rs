use core::ffi::{CStr, c_void};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use automata_game::address::CREATE_EVENT_IMPORT_SLOT;
use automata_game::build::{SUPPORTED_PE_IDENTITIES, variant_of_image_size};
use automata_game::second_copy::{event_treatment, must_rename, unique_event_name};
use nt_hook::build_gate::VerifiedBuild;
use nt_hook::diagnostics::log_line;
use nt_hook::memory::{GameMemory, InProcessMemory};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, SetLastError};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;

const STATE_NOT_TRIED: u8 = 0;
const STATE_INSTALLED: u8 = 1;
const STATE_UNKNOWN_EXE: u8 = 2;
const STATE_SLOT_UNREADABLE: u8 = 3;
const STATE_SLOT_EMPTY: u8 = 4;
const STATE_SLOT_NOT_WRITABLE: u8 = 5;

static INSTALL_STATE: AtomicU8 = AtomicU8::new(STATE_NOT_TRIED);
static REAL_CREATE_EVENT: AtomicUsize = AtomicUsize::new(0);

type CreateEventA = unsafe extern "system" fn(*const c_void, i32, i32, *const u8) -> HANDLE;

pub fn install_second_copy_guard() {
    INSTALL_STATE.store(patch_import_slot(), Ordering::Release);
}

pub fn log_second_copy_state() {
    let text = match INSTALL_STATE.load(Ordering::Acquire) {
        STATE_INSTALLED => {
            "second copy: CreateEventA import slot replaced (a second copy gets its own event name; a guess until two copies run)"
        }
        STATE_UNKNOWN_EXE => "second copy: the exe is not a known build, import slot left alone",
        STATE_SLOT_UNREADABLE => {
            "second copy: cannot read the CreateEventA import slot, not installed"
        }
        STATE_SLOT_EMPTY => "second copy: the CreateEventA import slot is empty, not installed",
        STATE_SLOT_NOT_WRITABLE => {
            "second copy: cannot write the CreateEventA import slot, not installed"
        }
        _ => "second copy: guard was never started",
    };
    log_line(text);
}

fn patch_import_slot() -> u8 {
    let Ok(build) = VerifiedBuild::verify(&SUPPORTED_PE_IDENTITIES) else {
        return STATE_UNKNOWN_EXE;
    };
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        return STATE_UNKNOWN_EXE;
    };
    let slot = build.module_base() + u64::from(CREATE_EVENT_IMPORT_SLOT.rva(variant));
    let mut memory = InProcessMemory::new(&build);
    let Ok(real) = memory.read_u64(slot) else {
        return STATE_SLOT_UNREADABLE;
    };
    if real == 0 {
        return STATE_SLOT_EMPTY;
    }
    REAL_CREATE_EVENT.store(real as usize, Ordering::Release);
    let replacement = create_event_detour as *const () as usize as u64;
    if memory.write(slot, &replacement.to_le_bytes()).is_err() {
        REAL_CREATE_EVENT.store(0, Ordering::Release);
        return STATE_SLOT_NOT_WRITABLE;
    }
    STATE_INSTALLED
}

#[expect(
    unsafe_code,
    reason = "replaces the game's CreateEventA import; the signature is the Win32 one"
)]
unsafe extern "system" fn create_event_detour(
    attributes: *const c_void,
    manual_reset: i32,
    initial_state: i32,
    name: *const u8,
) -> HANDLE {
    // SAFETY: the slot held a function pointer to the real CreateEventA, stored before this
    // function was put in the slot.
    let real = unsafe {
        core::mem::transmute::<usize, CreateEventA>(REAL_CREATE_EVENT.load(Ordering::Acquire))
    };
    // SAFETY: the arguments are the caller's own, passed through unchanged.
    let handle = unsafe { real(attributes, manual_reset, initial_state, name) };
    // SAFETY: reads the last error of the call just made on this thread.
    let last_error = unsafe { GetLastError() };
    let given_name = if name.is_null() {
        None
    } else {
        // SAFETY: a non-null name given to CreateEventA is a NUL-terminated string.
        Some(unsafe { CStr::from_ptr(name.cast()) }.to_bytes())
    };
    if !must_rename(event_treatment(given_name), last_error) {
        return handle;
    }
    let mut unique = unique_event_name(
        // SAFETY: GetCurrentProcessId takes no arguments and cannot fail.
        unsafe { GetCurrentProcessId() },
    )
    .into_bytes();
    unique.push(0);
    // SAFETY: closes the handle the call above returned for the already existing event.
    unsafe { CloseHandle(handle) };
    // SAFETY: `unique` is a NUL-terminated string that outlives the call.
    let renamed = unsafe { real(attributes, manual_reset, initial_state, unique.as_ptr()) };
    // SAFETY: sets the last error of this thread, which the game reads right after this call.
    unsafe { SetLastError(0) };
    renamed
}
