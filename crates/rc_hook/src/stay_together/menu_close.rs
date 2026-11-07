use core::ffi::c_void;

use rc_game::GameMemory;
use rc_game::marker::UI_DIALOG_STATE_RVA;
use rc_game::offsets::CLOSE_MAIN_MENU_FUNCTION_RVA;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;

type CloseMainMenu = unsafe extern "system" fn(*mut c_void, u8);

pub(crate) fn main_menu_open(build: &VerifiedBuild) -> bool {
    InProcessMemory::new(build)
        .read_i32(build.module_base() + u64::from(UI_DIALOG_STATE_RVA))
        .is_ok_and(|state| state != 0)
}

pub(crate) fn close_main_menu(build: &VerifiedBuild, why: &str) {
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "calls the game's own main menu close at its verified address"
    )]
    // SAFETY: the build gate passed, so the RVA starts the menu close function (menu object, flag)
    // and the object address is the menu object; this runs on the game thread in the script tick.
    unsafe {
        let close: CloseMainMenu = core::mem::transmute::<usize, CloseMainMenu>(
            (base + u64::from(CLOSE_MAIN_MENU_FUNCTION_RVA)) as usize,
        );
        close((base + u64::from(UI_DIALOG_STATE_RVA)) as *mut c_void, 1);
    }
    log_line(&format!("menu: closed the pause menu ({why})"));
}
