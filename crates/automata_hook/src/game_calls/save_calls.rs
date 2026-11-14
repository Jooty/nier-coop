use automata_game::address::{
    APPLY_LOADED_STORY_FUNCTION, COMPAT_APPLY_LOADED_STORY_FUNCTION_BYTES,
    COMPAT_SAVE_LOAD_FAILED_FUNCTION_BYTES, REQUEST_LOAD_SLOT_FUNCTION,
    REQUEST_PHASE_CHANGE_FUNCTION, SAVE_LOAD_FAILED_FUNCTION, SAVE_MANAGER_IS_BUSY_FUNCTION,
    SCENE_OBJECT,
};
use automata_game::exe::ExeVariant;

use crate::game_calls::GameCalls;

type FlagQuery = unsafe extern "system" fn() -> usize;
type RequestLoadSlot = unsafe extern "system" fn(usize) -> usize;
type RequestPhaseChange = unsafe extern "system" fn(usize, usize, usize, *const i32) -> usize;

impl GameCalls {
    /// The two Compat addresses were found by byte shape, so they are used only when the bytes
    /// there are the expected ones.
    pub fn slot_calls_verified(&self) -> bool {
        if self.variant != ExeVariant::Compat {
            return true;
        }
        let matches = |address, expected: &[u8]| {
            self.read_bytes(address, expected.len())
                .is_ok_and(|bytes| bytes == expected)
        };
        matches(
            SAVE_LOAD_FAILED_FUNCTION,
            &COMPAT_SAVE_LOAD_FAILED_FUNCTION_BYTES,
        ) && matches(
            APPLY_LOADED_STORY_FUNCTION,
            &COMPAT_APPLY_LOADED_STORY_FUNCTION_BYTES,
        )
    }

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
        if !self.slot_calls_verified() {
            return None;
        }
        // SAFETY: the address is the failure query of the verified build (on Compat its bytes
        // were just checked); it takes no arguments.
        let result = unsafe {
            core::mem::transmute::<usize, FlagQuery>(self.address_of(SAVE_LOAD_FAILED_FUNCTION))()
        };
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

    #[expect(
        unsafe_code,
        reason = "calls the game's apply-the-loaded-story request"
    )]
    pub fn apply_loaded_story(&self) -> bool {
        if !self.slot_calls_verified() {
            return false;
        }
        // SAFETY: the address is the save manager's apply request of the verified build (on
        // Compat its bytes were just checked); it takes no arguments and returns a bool in al.
        let result = unsafe {
            core::mem::transmute::<usize, FlagQuery>(self.address_of(APPLY_LOADED_STORY_FUNCTION))()
        };
        result & 0xFF != 0
    }

    #[expect(unsafe_code, reason = "calls the game's scene phase change request")]
    pub fn request_saved_scene(&self, phase_id: i32, name_address: usize, parameters: &[i32; 6]) {
        // SAFETY: the address is Scene::RequestPhaseChange of the verified build; the scene object
        // is the game's global, the name points into the game's own saved-phase buffer, and the
        // parameter block is borrowed for the call.
        unsafe {
            core::mem::transmute::<usize, RequestPhaseChange>(
                self.address_of(REQUEST_PHASE_CHANGE_FUNCTION),
            )(
                self.address_of(SCENE_OBJECT),
                phase_id as u32 as usize,
                name_address,
                parameters.as_ptr(),
            );
        }
    }
}
