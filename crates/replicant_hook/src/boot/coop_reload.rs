use core::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use replicant_game::GameMemory;
use replicant_game::save_files::{ReloadInputs, ReloadProgress, reload_progress};

use crate::boot::game_access::{SAVE_FILE_IMAGE_RVA, game_memory};
use crate::diagnostics::log_line;
use crate::net::net_clock_ms;

const SAVE_MANAGER_RVA: u64 = 0x176FAD0;
const SAVE_MANAGER_STATE_OFFSET: u64 = 0x48260;
const SAVE_MANAGER_RESULT_OFFSET: u64 = 0x48264;
const REQUEST_LOAD_RVA: u64 = 0x7B9E0;
const SAVE_HEADER_COPY_RVA: u64 = 0x11FDF80;
const SAVE_HEADER_COPY_BYTES: usize = 0x8158;

const NOT_YET: u64 = u64::MAX;

static REQUESTED: AtomicBool = AtomicBool::new(false);
static BUSY_SEEN: AtomicBool = AtomicBool::new(false);
static FIRST_STEP_MS: AtomicU64 = AtomicU64::new(NOT_YET);
static REQUESTED_MS: AtomicU64 = AtomicU64::new(NOT_YET);

type RequestLoadFn = unsafe extern "system" fn(*mut c_void);

pub(super) enum ReloadStep {
    Waiting,
    Loaded,
    Failed,
}

pub(super) fn reload_step() -> ReloadStep {
    let Some(mut memory) = game_memory() else {
        return ReloadStep::Failed;
    };
    let manager = memory.module_base() + SAVE_MANAGER_RVA;
    let (Ok(state), Ok(result)) = (
        memory.read_i32(manager + SAVE_MANAGER_STATE_OFFSET),
        memory.read_u8(manager + SAVE_MANAGER_RESULT_OFFSET),
    ) else {
        return ReloadStep::Failed;
    };
    let now = net_clock_ms();
    let first_step =
        match FIRST_STEP_MS.compare_exchange(NOT_YET, now, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => now,
            Err(first) => first,
        };
    if state != 0 {
        BUSY_SEEN.store(true, Ordering::Release);
    }
    let requested_ms = REQUESTED_MS.load(Ordering::Acquire);
    let progress = reload_progress(ReloadInputs {
        requested: REQUESTED.load(Ordering::Acquire),
        busy_seen: BUSY_SEEN.load(Ordering::Acquire),
        waited_ms: now.saturating_sub(first_step),
        since_request_ms: if requested_ms == NOT_YET {
            0
        } else {
            now.saturating_sub(requested_ms)
        },
        manager_state: state,
        result,
    });
    match progress {
        ReloadProgress::Request => {
            request_load(memory.module_base(), manager);
            REQUESTED_MS.store(now, Ordering::Release);
            REQUESTED.store(true, Ordering::Release);
            log_line("boot: reloading the save image through the redirected read");
            ReloadStep::Waiting
        }
        ReloadProgress::Wait => ReloadStep::Waiting,
        ReloadProgress::TimedOut => {
            log_line(&format!(
                "boot: the save image reload timed out (manager state {state}, result {result:#x})"
            ));
            ReloadStep::Failed
        }
        ReloadProgress::Failed(code) => {
            log_line(&format!(
                "boot: the save image reload failed (result {code:#x})"
            ));
            ReloadStep::Failed
        }
        ReloadProgress::Loaded => {
            if refresh_header_copy(&mut memory) {
                ReloadStep::Loaded
            } else {
                ReloadStep::Failed
            }
        }
    }
}

#[expect(
    unsafe_code,
    reason = "calls GameSaveDataManager::RequestLoad by its address"
)]
fn request_load(module_base: u64, manager: u64) {
    let function = (module_base + REQUEST_LOAD_RVA) as usize;
    // SAFETY: the RVA starts GameSaveDataManager::RequestLoad (manager) -> void in the verified
    // build; it only queues a read when the manager is idle, which the caller checked.
    unsafe {
        core::mem::transmute::<usize, RequestLoadFn>(function)(manager as usize as *mut c_void);
    }
}

fn refresh_header_copy<M: GameMemory>(memory: &mut M) -> bool {
    // The boot read copied the header once; the slot-valid mask lives inside that copy.
    let image = memory.module_base() + SAVE_FILE_IMAGE_RVA;
    let copy = memory.module_base() + SAVE_HEADER_COPY_RVA;
    let mut header = vec![0_u8; SAVE_HEADER_COPY_BYTES];
    if memory.read(image, &mut header).is_ok() && memory.write(copy, &header).is_ok() {
        log_line("boot: save image reloaded from the co-op file, slot mask refreshed");
        true
    } else {
        log_line("boot: save image reloaded but the slot mask could not be refreshed");
        false
    }
}
