use core::ffi::c_void;
use core::ptr;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, Ordering};

use rc_game::{GameMemory, MemoryError};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;

const START_TITLE_RVA: u64 = 0x70AE10;
const START_LOAD_RVA: u64 = 0x708DC0;
const MENU_RESULT_RVA: u64 = 0x484CF80;
const SAVE_FILE_IMAGE_RVA: u64 = 0x4386EE0;
const COPY_SLOT_TO_BLOCK_RVA: u64 = 0x3BE800;
const SLOT_VALID_MASK_RVA: u64 = 0x11FDFEC;
const RESULT_CANCELLED: i32 = 2;
const RESULT_LOAD_GAME: i32 = 5;
const RESULT_DONE: i32 = 6;
const NATIVE_FINISHED: i32 = 0;
const NO_BOOT_SLOT: i32 = -1;

static BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_START_TITLE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_START_LOAD: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static TITLE_SKIP_PENDING: AtomicBool = AtomicBool::new(false);
static SLOT_COPY_PENDING: AtomicBool = AtomicBool::new(false);
static BOOT_SLOT: AtomicI32 = AtomicI32::new(NO_BOOT_SLOT);

type NativeFn = unsafe extern "system" fn(*mut c_void) -> i32;
type CopySlotToBlockFn = unsafe extern "system" fn(*mut c_void, i32) -> u8;

pub fn install(build: VerifiedBuild, slot: u8) {
    if BUILD.set(build).is_err() {
        return;
    }
    BOOT_SLOT.store(i32::from(slot), Ordering::Release);
    TITLE_SKIP_PENDING.store(true, Ordering::Release);
    hook_native(
        "_StartTitle",
        START_TITLE_RVA,
        start_title_detour as *const () as *mut c_void,
        &ORIGINAL_START_TITLE,
    );
    hook_native(
        "_StartLoad",
        START_LOAD_RVA,
        start_load_detour as *const () as *mut c_void,
        &ORIGINAL_START_LOAD,
    );
    log_line(&format!("boot: going straight to save slot {slot}"));
}

fn hook_native(label: &str, rva: u64, replacement: *mut c_void, original: &AtomicPtr<c_void>) {
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

fn game_memory() -> Option<InProcessMemory> {
    BUILD.get().map(InProcessMemory::new)
}

fn write_menu_result(result: i32) -> Result<(), MemoryError> {
    let mut memory = game_memory().ok_or(MemoryError::Unreadable { address: 0 })?;
    let address = memory.module_base() + MENU_RESULT_RVA;
    memory.write(address, &result.to_le_bytes())
}

fn slot_holds_a_save(slot: i32) -> bool {
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

#[expect(unsafe_code, reason = "calls SaveData::CopySlotToBlock by its address")]
fn copy_slot_to_block(slot: i32) -> bool {
    let Some(build) = BUILD.get() else {
        return false;
    };
    let function = (build.module_base() + COPY_SLOT_TO_BLOCK_RVA) as usize;
    let image = (build.module_base() + SAVE_FILE_IMAGE_RVA) as usize as *mut c_void;
    // SAFETY: the RVA starts SaveData::CopySlotToBlock (image, slot) -> bool in the verified build.
    let copied = unsafe { core::mem::transmute::<usize, CopySlotToBlockFn>(function)(image, slot) };
    copied != 0
}

#[expect(
    unsafe_code,
    reason = "calls the original native through the trampoline"
)]
fn call_original(original: &AtomicPtr<c_void>, argument: *mut c_void) -> i32 {
    let original = original.load(Ordering::Acquire);
    if original.is_null() {
        return NATIVE_FINISHED;
    }
    // SAFETY: the trampoline continues into the original native, which has this signature.
    unsafe { core::mem::transmute::<*mut c_void, NativeFn>(original)(argument) }
}

extern "system" fn start_title_detour(argument: *mut c_void) -> i32 {
    if !TITLE_SKIP_PENDING.swap(false, Ordering::AcqRel) {
        return call_original(&ORIGINAL_START_TITLE, argument);
    }
    let slot = BOOT_SLOT.load(Ordering::Acquire);
    if !slot_holds_a_save(slot) {
        log_line(&format!(
            "boot: slot {slot} holds no save, showing the title"
        ));
        return call_original(&ORIGINAL_START_TITLE, argument);
    }
    if let Err(error) = write_menu_result(RESULT_LOAD_GAME) {
        log_line(&format!("boot: cannot skip the title ({error})"));
        return call_original(&ORIGINAL_START_TITLE, argument);
    }
    SLOT_COPY_PENDING.store(true, Ordering::Release);
    log_line("boot: title skipped");
    NATIVE_FINISHED
}

extern "system" fn start_load_detour(argument: *mut c_void) -> i32 {
    if !SLOT_COPY_PENDING.swap(false, Ordering::AcqRel) {
        return call_original(&ORIGINAL_START_LOAD, argument);
    }
    let slot = BOOT_SLOT.load(Ordering::Acquire);
    let loaded = copy_slot_to_block(slot);
    log_line(&format!("boot: slot {slot} copied={loaded}"));
    let result = if loaded {
        RESULT_DONE
    } else {
        RESULT_CANCELLED
    };
    let _ = write_menu_result(result);
    NATIVE_FINISHED
}
