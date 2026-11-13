use automata_game::address::{
    CURRENT_PHASE_ID_VALUE, CURRENT_SUB_PHASE_NAME_VALUE, PENDING_PHASE_ID_VALUE,
    PENDING_SUB_PHASE_NAME_VALUE, PHASE_JUMP_FLAG_WORD_VALUE, PHASE_SYSTEM_STATE_VALUE,
    REQUEST_PHASE_CHANGE_FUNCTION, REQUESTED_PHASE_ID_VALUE, SCENE_OBJECT, SCENE_STATE_VALUE,
    SET_PENDING_SUB_PHASE_FUNCTION,
};
use automata_game::start::{SUB_PHASE_NAME_BUFFER_LEN, SubPhaseReading};
use automata_game::story_follow::{
    PHASE_JUMP_FLAG_WORD_MASK_AFTER_A_SUB_PHASE_REQUEST, RequestReadiness, StoryRequest,
};

use crate::game_calls::GameCalls;

const NOT_READ: i32 = -99;

type RequestPhaseChange = unsafe extern "system" fn(usize, usize, usize, *const i32) -> usize;
type SetPendingSubPhase = unsafe extern "system" fn(usize, usize);

impl GameCalls {
    pub fn request_readiness(&self) -> RequestReadiness {
        RequestReadiness {
            scene_state: self.read_i32(SCENE_STATE_VALUE).unwrap_or(NOT_READ),
            phase_system_state: self.read_i32(PHASE_SYSTEM_STATE_VALUE).unwrap_or(NOT_READ),
            requested_phase_id: self.read_i32(REQUESTED_PHASE_ID_VALUE).unwrap_or(NOT_READ),
            pending_phase_id: self.read_i32(PENDING_PHASE_ID_VALUE).unwrap_or(NOT_READ),
            current_phase_id: self.read_i32(CURRENT_PHASE_ID_VALUE).unwrap_or(NOT_READ),
            pending_name: self.sub_phase_name_at(PENDING_SUB_PHASE_NAME_VALUE),
            current_name: self.sub_phase_name_at(CURRENT_SUB_PHASE_NAME_VALUE),
        }
    }

    pub fn make_story_request(&self, request: &StoryRequest) {
        match request {
            StoryRequest::Full { id, name, params } => {
                self.make_full_phase_change(*id, name, params)
            }
            StoryRequest::SubPhase { id, name } => self.make_sub_phase_change(*id, name),
        }
    }

    fn sub_phase_name_at(&self, address: automata_game::address::GameAddress) -> Option<String> {
        let bytes = self.read_bytes(address, SUB_PHASE_NAME_BUFFER_LEN).ok()?;
        match SubPhaseReading::of_bytes(&bytes) {
            SubPhaseReading::Name(name) => Some(name),
            SubPhaseReading::NotAName => None,
        }
    }

    #[expect(unsafe_code, reason = "calls the game's scene phase change request")]
    fn make_full_phase_change(&self, id: i32, name: &str, params: &[i32; 6]) {
        let mut name_with_nul = name.as_bytes().to_vec();
        name_with_nul.push(0);
        // An empty name is the NULL name: the phase's first sub-phase.
        let name_address = if name.is_empty() {
            0
        } else {
            name_with_nul.as_ptr() as usize
        };
        // SAFETY: the address is Scene::RequestPhaseChange of the verified build, the scene object
        // is the game's global, the name is a NUL-terminated buffer that outlives the call (the game
        // copies it) and the parameter block is six numbers. The caller is the game thread with the
        // scene running and the phase system idle.
        unsafe {
            core::mem::transmute::<usize, RequestPhaseChange>(
                self.address_of(REQUEST_PHASE_CHANGE_FUNCTION),
            )(
                self.address_of(SCENE_OBJECT),
                id as u32 as usize,
                name_address,
                params.as_ptr(),
            );
        }
    }

    #[expect(unsafe_code, reason = "calls the game's pending sub-phase setter")]
    fn make_sub_phase_change(&self, id: i32, name: &str) {
        let mut name_with_nul = name.as_bytes().to_vec();
        name_with_nul.push(0);
        // SAFETY: the address is Phase::SetPendingSubPhase of the verified build; it copies the
        // name, which is a NUL-terminated buffer that outlives the call.
        unsafe {
            core::mem::transmute::<usize, SetPendingSubPhase>(
                self.address_of(SET_PENDING_SUB_PHASE_FUNCTION),
            )(id as u32 as usize, name_with_nul.as_ptr() as usize);
        }
        // The game's own callers clear these bits after the setter; the setter does not.
        if let Ok(word) = self.read_u32(PHASE_JUMP_FLAG_WORD_VALUE) {
            let _ = self.write_u32(
                PHASE_JUMP_FLAG_WORD_VALUE,
                word & PHASE_JUMP_FLAG_WORD_MASK_AFTER_A_SUB_PHASE_REQUEST,
            );
        }
    }
}
