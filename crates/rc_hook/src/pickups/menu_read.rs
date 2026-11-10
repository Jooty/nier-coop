use core::ffi::c_void;
use core::ptr;
use std::sync::{Mutex, PoisonError};

use windows_sys::Win32::System::Diagnostics::Debug::RtlCaptureStackBackTrace;

use rc_game::pickups::is_menu_quest_page_caller;

use crate::diagnostics::log_line;
use crate::pickups::item_calls::hooked_build;
use crate::stay_together::pause_menu_open;

const CAPTURED_FRAMES: usize = 8;
const GAME_IMAGE_RVA_LIMIT: u64 = 0x1000_0000;

static LOGGED_MENU_READS: Mutex<Vec<(u32, i32)>> = Mutex::new(Vec::new());

#[expect(
    unsafe_code,
    reason = "walks the stack to find which game function reads the count"
)]
pub fn note_count_read(item_id: i32) -> bool {
    let Some(build) = hooked_build() else {
        return false;
    };
    if !pause_menu_open(build) {
        return false;
    }
    let mut frames = [ptr::null_mut::<c_void>(); CAPTURED_FRAMES];
    // SAFETY: the buffer holds the frames asked for; the hash out-pointer may be null.
    let captured = unsafe {
        RtlCaptureStackBackTrace(
            0,
            CAPTURED_FRAMES as u32,
            frames.as_mut_ptr(),
            ptr::null_mut(),
        )
    };
    let base = build.module_base();
    let Some(rva) = frames
        .iter()
        .take(usize::from(captured))
        .map(|frame| (*frame as usize as u64).wrapping_sub(base))
        .find(|rva| *rva < GAME_IMAGE_RVA_LIMIT)
    else {
        return false;
    };
    let rva = rva as u32;
    log_first_read(item_id, rva);
    is_menu_quest_page_caller(rva)
}

fn log_first_read(item_id: i32, rva: u32) {
    let mut logged = LOGGED_MENU_READS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if logged.contains(&(rva, item_id)) {
        return;
    }
    logged.push((rva, item_id));
    log_line(&format!(
        "shared items: menu read of {item_id} from rva {rva:#x}"
    ));
}
