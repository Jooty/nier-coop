use core::ffi::c_void;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

use rc_game::GameMemory;
use rc_game::marker::MAIN_MENU_STATE_RVA;
use rc_game::offsets::CLOSE_MAIN_MENU_FUNCTION_RVA;
use rc_game::stay_together::{MenuGuard, MenuGuardInput};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::game_hooks::{keep_running_enabled, window_unfocused};
use crate::memory::InProcessMemory;

type CloseMainMenu = unsafe extern "system" fn(*mut c_void, u8);

static STARTED_AT: OnceLock<Instant> = OnceLock::new();
static GUARD: Mutex<MenuGuard> = Mutex::new(MenuGuard::new());

pub(super) fn main_menu_open(build: &VerifiedBuild) -> bool {
    InProcessMemory::new(build)
        .read_i32(build.module_base() + u64::from(MAIN_MENU_STATE_RVA))
        .is_ok_and(|state| state != 0)
}

pub(super) fn close_main_menu(build: &VerifiedBuild, why: &str) {
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
        close((base + u64::from(MAIN_MENU_STATE_RVA)) as *mut c_void, 1);
    }
    log_line(&format!("menu: closed the pause menu ({why})"));
}

pub(super) fn close_menu_opened_by_focus_change(build: &VerifiedBuild) {
    let now_ms = STARTED_AT.get_or_init(Instant::now).elapsed().as_millis() as u64;
    let close = GUARD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .should_close_menu(&MenuGuardInput {
            keep_running: keep_running_enabled(),
            unfocused: window_unfocused(),
            menu_open: main_menu_open(build),
            now_ms,
        });
    if close {
        close_main_menu(
            build,
            "opened by losing window focus while keep running is on",
        );
    }
}
