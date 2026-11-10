use core::ffi::c_void;

use replicant_game::offsets::{
    UI_MANAGER_GET_FUNCTION_RVA, UI_MANAGER_IS_FREE_TO_ACT_FUNCTION_RVA,
};

use crate::build_gate::VerifiedBuild;

type UiManagerGet = unsafe extern "system" fn() -> *mut c_void;
type UiManagerIsFreeToAct = unsafe extern "system" fn(*mut c_void) -> u8;

pub(crate) fn game_says_free_to_act(build: &VerifiedBuild) -> Option<bool> {
    let base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "calls the game's own free-to-act test at its verified address"
    )]
    // SAFETY: the build gate passed, so each RVA starts the function named in symbols.tsv with the
    // signature declared above; this runs on the game thread inside the script manager tick.
    let free = unsafe {
        let get: UiManagerGet = core::mem::transmute::<usize, UiManagerGet>(
            (base + u64::from(UI_MANAGER_GET_FUNCTION_RVA)) as usize,
        );
        let is_free: UiManagerIsFreeToAct = core::mem::transmute::<usize, UiManagerIsFreeToAct>(
            (base + u64::from(UI_MANAGER_IS_FREE_TO_ACT_FUNCTION_RVA)) as usize,
        );
        let ui_manager = get();
        if ui_manager.is_null() {
            return None;
        }
        is_free(ui_manager)
    };
    Some(free != 0)
}
