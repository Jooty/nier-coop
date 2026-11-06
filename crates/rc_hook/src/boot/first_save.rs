use core::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::new_game::{FirstSaveGate, FirstSaveInputs, FirstSaveStep};
use rc_game::readiness::{classify, read_play_checks};
use rc_game::{GameMemory, Replicant};

use crate::boot::game_access::{SAVE_FILE_IMAGE_RVA, game_memory};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;

const SAVE_MANAGER_RVA: u64 = 0x176FAD0;
const SAVE_MANAGER_RESULT_OFFSET: u64 = 0x48264;
const COPY_BLOCK_TO_SLOT_RVA: u64 = 0x3BE880;
const FIX_CHECKSUMS_BEFORE_SAVE_RVA: u64 = 0x3BEA20;
const USER_MANAGER_ACCESSOR_RVA: u64 = 0x75660;
const USER_ENTRY_RVA: u64 = 0x7BEB00;
const SET_SAVE_USER_RVA: u64 = 0x7B960;
const REQUEST_SAVE_RVA: u64 = 0x7BA80;
const IS_LOADING_RVA: u64 = 0x7B9C0;
const IS_SAVING_RVA: u64 = 0x7B9D0;
const LAST_SAVED_SLOT_RVA: u64 = 0x1207288;
const WRITE_SUCCEEDED: u8 = 0;
const NO_SLOT: i32 = -1;

static GATE: Mutex<FirstSaveGate> = Mutex::new(FirstSaveGate::new());
static SLOT: AtomicI32 = AtomicI32::new(NO_SLOT);
static WRITE_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

type CopyBlockToSlotFn = unsafe extern "system" fn(*mut c_void, u32, u8, u8);
type FixChecksumsFn = unsafe extern "system" fn(*mut c_void);
type AccessorFn = unsafe extern "system" fn() -> *mut c_void;
type UserEntryFn = unsafe extern "system" fn(*mut c_void) -> *mut c_void;
type SetSaveUserFn = unsafe extern "system" fn(*mut c_void, *mut c_void);
type RequestSaveFn = unsafe extern "system" fn(*mut c_void);
type ManagerQueryFn = unsafe extern "system" fn(*mut c_void) -> u8;

pub(super) fn arm(slot: u8) {
    SLOT.store(i32::from(slot), Ordering::Release);
    GATE.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .arm(net_clock_ms());
    log_line(&format!("first save: armed for slot {slot}"));
}

pub fn tick(build: &VerifiedBuild) {
    let slot = SLOT.load(Ordering::Acquire);
    if slot == NO_SLOT {
        return;
    }
    if WRITE_IN_FLIGHT.load(Ordering::Acquire) {
        finish_write_when_done(build, slot);
        return;
    }
    let game = Replicant::new(InProcessMemory::new(build));
    let phase = game.current_phase().unwrap_or_default();
    let inputs = FirstSaveInputs {
        in_play: classify(read_play_checks(&game)).is_in_play(),
        phase: &phase,
        save_manager_busy: manager_is_loading(build) || manager_is_saving(build),
    };
    let step = GATE
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .step(inputs, net_clock_ms());
    match step {
        FirstSaveStep::Save => request_first_save(build, slot),
        FirstSaveStep::GiveUp => {
            log_line(&format!(
                "first save: slot {slot} given up, the first phase never came"
            ));
            SLOT.store(NO_SLOT, Ordering::Release);
        }
        FirstSaveStep::Wait | FirstSaveStep::Done => {}
    }
}

fn request_first_save(build: &VerifiedBuild, slot: i32) {
    let Ok(slot_number) = u32::try_from(slot) else {
        return;
    };
    call_save_sequence(build, slot_number);
    WRITE_IN_FLIGHT.store(true, Ordering::Release);
    log_line(&format!("first save: slot {slot} requested"));
}

fn finish_write_when_done(build: &VerifiedBuild, slot: i32) {
    if manager_is_saving(build) {
        return;
    }
    let result = save_manager_result(build);
    log_line(&format!(
        "first save: slot {slot} written, result {result:?}"
    ));
    if result == Some(WRITE_SUCCEEDED) {
        mirror_last_saved_slot(slot);
    }
    WRITE_IN_FLIGHT.store(false, Ordering::Release);
    SLOT.store(NO_SLOT, Ordering::Release);
}

fn save_manager(build: &VerifiedBuild) -> *mut c_void {
    (build.module_base() + SAVE_MANAGER_RVA) as usize as *mut c_void
}

fn save_manager_result(build: &VerifiedBuild) -> Option<u8> {
    let memory = InProcessMemory::new(build);
    memory
        .read_u8(build.module_base() + SAVE_MANAGER_RVA + SAVE_MANAGER_RESULT_OFFSET)
        .ok()
}

fn mirror_last_saved_slot(slot: i32) {
    let Some(mut memory) = game_memory() else {
        return;
    };
    let address = memory.module_base() + LAST_SAVED_SLOT_RVA;
    let _ = memory.write(address, &slot.to_le_bytes());
}

#[expect(
    unsafe_code,
    reason = "calls GameSaveDataManager::IsLoading by its address"
)]
fn manager_is_loading(build: &VerifiedBuild) -> bool {
    let function = (build.module_base() + IS_LOADING_RVA) as usize;
    // SAFETY: the RVA starts GameSaveDataManager::IsLoading (manager) -> bool in the verified build.
    unsafe { core::mem::transmute::<usize, ManagerQueryFn>(function)(save_manager(build)) != 0 }
}

#[expect(
    unsafe_code,
    reason = "calls GameSaveDataManager::IsSaving by its address"
)]
fn manager_is_saving(build: &VerifiedBuild) -> bool {
    let function = (build.module_base() + IS_SAVING_RVA) as usize;
    // SAFETY: the RVA starts GameSaveDataManager::IsSaving (manager) -> bool in the verified build.
    unsafe { core::mem::transmute::<usize, ManagerQueryFn>(function)(save_manager(build)) != 0 }
}

#[expect(
    unsafe_code,
    reason = "calls the slot menu's own save sequence by address"
)]
fn call_save_sequence(build: &VerifiedBuild, slot: u32) {
    let base = build.module_base();
    let image = (base + SAVE_FILE_IMAGE_RVA) as usize as *mut c_void;
    let manager = save_manager(build);
    // SAFETY: each RVA starts the function named in its constant in the verified build, with the
    // signature of its type alias; the order and arguments are those of the slot menu's save
    // state (SaveloadMenuRootSequence, case 2), run on the game thread with the manager idle.
    unsafe {
        core::mem::transmute::<usize, CopyBlockToSlotFn>((base + COPY_BLOCK_TO_SLOT_RVA) as usize)(
            image, slot, 0, 0,
        );
        core::mem::transmute::<usize, FixChecksumsFn>(
            (base + FIX_CHECKSUMS_BEFORE_SAVE_RVA) as usize,
        )(image);
        let user_manager = core::mem::transmute::<usize, AccessorFn>(
            (base + USER_MANAGER_ACCESSOR_RVA) as usize,
        )();
        let user = core::mem::transmute::<usize, UserEntryFn>((base + USER_ENTRY_RVA) as usize)(
            user_manager,
        );
        core::mem::transmute::<usize, SetSaveUserFn>((base + SET_SAVE_USER_RVA) as usize)(
            manager, user,
        );
        core::mem::transmute::<usize, RequestSaveFn>((base + REQUEST_SAVE_RVA) as usize)(manager);
    }
}
