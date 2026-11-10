use core::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use replicant_game::{GameMemory, MemoryError};

use crate::build_gate::VerifiedBuild;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

pub(super) const START_TITLE_RVA: u64 = 0x70AE10;
pub(super) const START_INPUT_PLAYER_NAME_RVA: u64 = 0x7086E0;
pub(super) const SAVE_FILE_IMAGE_RVA: u64 = 0x4386EE0;
pub(super) const NATIVE_FINISHED: i32 = 0;
pub(super) const NATIVE_STILL_RUNNING: i32 = 1;
const MENU_RESULT_RVA: u64 = 0x484CF80;
const SLOT_VALID_MASK_RVA: u64 = 0x11FDFEC;
const SYSTEM_DATA_LOADED_RVA: u64 = 0x1207280;

static BUILD: OnceLock<VerifiedBuild> = OnceLock::new();

pub(super) type NativeFn = unsafe extern "system" fn(*mut c_void) -> i32;

pub(super) fn remember_build(build: VerifiedBuild) -> bool {
    BUILD.set(build).is_ok()
}

pub(super) fn build() -> Option<&'static VerifiedBuild> {
    BUILD.get()
}

pub(super) fn hook_native(
    label: &str,
    rva: u64,
    replacement: *mut c_void,
    original: &AtomicPtr<c_void>,
) {
    let Some(build) = BUILD.get() else {
        return;
    };
    let address = (build.module_base() + rva) as usize;
    #[expect(unsafe_code, reason = "hooks a Lua native by its address")]
    // SAFETY: the RVA starts a Lua native (arg pointer in, polled i32 out) in the verified build,
    // and the replacement has the same signature.
    unsafe {
        hook_address(label, address, replacement, original)
    };
}

pub(super) fn game_memory() -> Option<InProcessMemory> {
    BUILD.get().map(InProcessMemory::new)
}

pub(super) fn write_menu_result(result: i32) -> Result<(), MemoryError> {
    let mut memory = game_memory().ok_or(MemoryError::Unreadable { address: 0 })?;
    let address = memory.module_base() + MENU_RESULT_RVA;
    memory.write(address, &result.to_le_bytes())
}

pub(super) fn slot_holds_a_save(slot: i32) -> bool {
    let Some(memory) = game_memory() else {
        return false;
    };
    let mut bytes = [0_u8; 4];
    if memory
        .read(memory.module_base() + SLOT_VALID_MASK_RVA, &mut bytes)
        .is_err()
    {
        return false;
    }
    u32::from_le_bytes(bytes) & (1_u32 << slot) != 0
}

pub(super) fn system_data_loaded() -> bool {
    let Some(memory) = game_memory() else {
        return false;
    };
    memory
        .read_u8(memory.module_base() + SYSTEM_DATA_LOADED_RVA)
        .is_ok_and(|flag| flag != 0)
}

#[expect(
    unsafe_code,
    reason = "calls the original native through the trampoline"
)]
pub(super) fn call_original(original: &AtomicPtr<c_void>, argument: *mut c_void) -> i32 {
    let original = original.load(Ordering::Acquire);
    if original.is_null() {
        return NATIVE_FINISHED;
    }
    // SAFETY: the trampoline continues into the original native, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, NativeFn>(original)(argument) }
}
