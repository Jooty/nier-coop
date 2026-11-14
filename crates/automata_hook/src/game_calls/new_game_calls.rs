use automata_game::address::{
    REQUEST_NEW_GAME_SYSTEM_SAVE_FUNCTION, REQUEST_PHASE_CHANGE_FUNCTION,
    REQUEST_SAVE_SLOT_FUNCTION, RESET_GAME_DATA_FUNCTION, SCENE_FADE_COLOUR_VALUE, SCENE_OBJECT,
    SET_CURRENT_SAVE_SLOT_FUNCTION, SET_DIFFICULTY_FUNCTION,
};
use std::sync::LazyLock;

use automata_game::start::{
    NEW_GAME_OPENING_PHASE_NAME, NEW_GAME_SCENE_REQUEST_PARAMETERS, NEW_GAME_WHITE_FADE_COLOUR,
    OPENING_MISSION_PHASE_ID,
};

use crate::game_calls::GameCalls;

static OPENING_PHASE_NAME_WITH_NUL: LazyLock<Vec<u8>> = LazyLock::new(|| {
    let mut name = NEW_GAME_OPENING_PHASE_NAME.as_bytes().to_vec();
    name.push(0);
    name
});

type NoArgumentCall = unsafe extern "system" fn();
type OneNumberCall = unsafe extern "system" fn(usize);
type RequestPhaseChange = unsafe extern "system" fn(usize, usize, usize, *const i32) -> usize;

impl GameCalls {
    /// The title's New Game in order: the game data is reset, the slot and the difficulty are set and
    /// the save manager writes the system data for the new game.
    #[expect(
        unsafe_code,
        reason = "calls the save manager's reset, slot, difficulty and new game requests"
    )]
    pub fn begin_new_game(&self, slot: u32, difficulty_index: u32) {
        // SAFETY: the four addresses are the save manager functions of the verified build that the
        // title calls for a new game; the reset takes nothing and the other three take one number.
        unsafe {
            core::mem::transmute::<usize, NoArgumentCall>(
                self.address_of(RESET_GAME_DATA_FUNCTION),
            )();
            core::mem::transmute::<usize, OneNumberCall>(
                self.address_of(SET_CURRENT_SAVE_SLOT_FUNCTION),
            )(slot as usize);
            core::mem::transmute::<usize, OneNumberCall>(self.address_of(SET_DIFFICULTY_FUNCTION))(
                difficulty_index as usize,
            );
            core::mem::transmute::<usize, OneNumberCall>(
                self.address_of(REQUEST_NEW_GAME_SYSTEM_SAVE_FUNCTION),
            )(slot as usize);
        }
    }

    #[expect(unsafe_code, reason = "calls the game's scene phase change request")]
    pub fn request_new_game_opening(&self) -> Result<(), String> {
        self.write_u32(SCENE_FADE_COLOUR_VALUE, NEW_GAME_WHITE_FADE_COLOUR)
            .map_err(|error| format!("a memory write failed ({error})"))?;
        // SAFETY: the address is Scene::RequestPhaseChange of the verified build; the scene object is
        // the game's global and the name is a static buffer and the parameter block a constant of this DLL.
        unsafe {
            core::mem::transmute::<usize, RequestPhaseChange>(
                self.address_of(REQUEST_PHASE_CHANGE_FUNCTION),
            )(
                self.address_of(SCENE_OBJECT),
                i32::from(OPENING_MISSION_PHASE_ID) as u32 as usize,
                OPENING_PHASE_NAME_WITH_NUL.as_ptr() as usize,
                NEW_GAME_SCENE_REQUEST_PARAMETERS.as_ptr(),
            );
        }
        Ok(())
    }

    #[expect(unsafe_code, reason = "calls the game's save slot write request")]
    pub fn request_save_slot(&self, slot: u32) {
        // SAFETY: the address is SaveMgr::RequestSaveSlot of the verified build; it takes the slot and
        // returns nothing. A slot of 4 or more is ignored by the game.
        unsafe {
            core::mem::transmute::<usize, OneNumberCall>(
                self.address_of(REQUEST_SAVE_SLOT_FUNCTION),
            )(slot as usize);
        }
    }
}
