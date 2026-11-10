use automata_game::address::{
    REQUEST_LOAD_SLOT_FUNCTION, REQUEST_PHASE_CHANGE_FUNCTION, SAVE_LOAD_FAILED_FUNCTION_MAIN_ONLY,
    SAVE_MANAGER_IS_BUSY_FUNCTION, SCENE_OBJECT,
};
use automata_game::exe::ExeVariant;

use crate::game_calls::GameCalls;

const SAVED_SCENE_REQUEST_PARAMETERS: [i32; 6] = [1, 1, 0x78, 1, 0, 0];

type FlagQuery = unsafe extern "system" fn() -> usize;
type RequestLoadSlot = unsafe extern "system" fn(usize) -> usize;
type RequestPhaseChange = unsafe extern "system" fn(usize, usize, usize, *const i32) -> usize;

impl GameCalls {
    #[expect(unsafe_code, reason = "calls the game's save manager busy query")]
    pub fn save_is_busy(&self) -> bool {
        // SAFETY: the address is SaveMgr::IsBusy of the verified build; it takes no arguments.
        let result = unsafe {
            core::mem::transmute::<usize, FlagQuery>(self.address_of(SAVE_MANAGER_IS_BUSY_FUNCTION))(
            )
        };
        result & 0xFF != 0
    }

    #[expect(unsafe_code, reason = "calls the game's save failure query")]
    pub fn save_failed(&self) -> Option<bool> {
        if self.variant != ExeVariant::Main {
            return None;
        }
        let address =
            self.build.module_base() as usize + SAVE_LOAD_FAILED_FUNCTION_MAIN_ONLY as usize;
        // SAFETY: the address is the failure query of the verified main exe (the notes give no
        // compat address, so compat never gets here); it takes no arguments.
        let result = unsafe { core::mem::transmute::<usize, FlagQuery>(address)() };
        Some(result & 0xFF != 0)
    }

    #[expect(unsafe_code, reason = "calls the game's save slot load request")]
    pub fn request_load_slot(&self, slot: u32) {
        // SAFETY: the address is SaveMgr::RequestLoadSlot of the verified build; it takes the slot.
        unsafe {
            core::mem::transmute::<usize, RequestLoadSlot>(
                self.address_of(REQUEST_LOAD_SLOT_FUNCTION),
            )(slot as usize);
        }
    }

    #[expect(unsafe_code, reason = "calls the game's scene phase change request")]
    pub fn request_saved_scene(&self, phase_id: i32, name_address: usize) {
        // SAFETY: the address is Scene::RequestPhaseChange of the verified build; the scene object
        // is the game's global, the name points into the game's own saved-phase buffer, and the
        // parameter block lives on this stack frame for the call.
        unsafe {
            core::mem::transmute::<usize, RequestPhaseChange>(
                self.address_of(REQUEST_PHASE_CHANGE_FUNCTION),
            )(
                self.address_of(SCENE_OBJECT),
                phase_id as u32 as usize,
                name_address,
                SAVED_SCENE_REQUEST_PARAMETERS.as_ptr(),
            );
        }
    }
}
