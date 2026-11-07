use core::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

use rc_game::GameMemory;
use rc_game::save_files::{ReloadProgress, reload_progress};

use crate::boot::game_access::{SAVE_FILE_IMAGE_RVA, game_memory};
use crate::diagnostics::log_line;

const SAVE_MANAGER_RVA: u64 = 0x176FAD0;
const SAVE_MANAGER_STATE_OFFSET: u64 = 0x48260;
const SAVE_MANAGER_RESULT_OFFSET: u64 = 0x48264;
const REQUEST_LOAD_RVA: u64 = 0x7B9E0;
const SAVE_HEADER_COPY_RVA: u64 = 0x11FDF80;
const SAVE_HEADER_COPY_BYTES: usize = 0x8158;

static REQUESTED: AtomicBool = AtomicBool::new(false);

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
    match reload_progress(REQUESTED.load(Ordering::Acquire), state, result) {
        ReloadProgress::Request => {
            request_load(memory.module_base(), manager);
            REQUESTED.store(true, Ordering::Release);
            log_line("boot: reloading the save image through the redirected read");
            ReloadStep::Waiting
        }
        ReloadProgress::Wait => ReloadStep::Waiting,
        ReloadProgress::Failed(code) => {
            log_line(&format!(
                "boot: the save image reload failed (result {code:#x})"
            ));
            ReloadStep::Failed
        }
        ReloadProgress::Loaded => {
            refresh_header_copy(&mut memory);
            ReloadStep::Loaded
        }
    }
}

#[expect(unsafe_code, reason = "calls GameSaveDataManager::RequestLoad by its address")]
fn request_load(module_base: u64, manager: u64) {
    let function = (module_base + REQUEST_LOAD_RVA) as usize;
    // SAFETY: the RVA starts GameSaveDataManager::RequestLoad (manager) -> void in the verified
    // build; it only queues a read when the manager is idle, which the caller checked.
    unsafe {
        core::mem::transmute::<usize, RequestLoadFn>(function)(manager as usize as *mut c_void);
    }
}

fn refresh_header_copy<M: GameMemory>(memory: &mut M) {
    // The boot read copied the header once; the slot-valid mask lives inside that copy.
    let image = memory.module_base() + SAVE_FILE_IMAGE_RVA;
    let copy = memory.module_base() + SAVE_HEADER_COPY_RVA;
    let mut header = vec![0_u8; SAVE_HEADER_COPY_BYTES];
    if memory.read(image, &mut header).is_ok() && memory.write(copy, &header).is_ok() {
        log_line("boot: save image reloaded from the co-op file, slot mask refreshed");
    } else {
        log_line("boot: save image reloaded but the slot mask could not be refreshed");
    }
}
