use automata_game::address::{
    FIND_STORY_ACTION_BY_ID_FUNCTION, SET_ACTION_STATE_BY_HASH_FUNCTION, STORY_HOST_OBJECT,
};
use automata_game::story_steps::{
    ACTION_DONE_STATE_HASH, ACTION_FLAGS_OFFSET, ACTION_STATE_VALUE_OFFSET, ActionLook,
    StepActionState,
};

use crate::game_calls::GameCalls;

type FindActionById = unsafe extern "system" fn(usize, u32) -> usize;
type SetStateByHash = unsafe extern "system" fn(usize, u32, i32) -> u8;

impl GameCalls {
    /// This copy's story action with the id and its state; the caller is the game thread.
    pub fn look_at_story_action(&self, id: u32) -> ActionLook {
        let Some(action) = self.story_action_by_id(id) else {
            return ActionLook {
                id,
                state: None,
                value: 0,
            };
        };
        let flags = self.read_u32_at((action + ACTION_FLAGS_OFFSET) as u64).ok();
        ActionLook {
            id,
            state: flags.map(StepActionState::of_flags),
            value: self
                .read_i32_at((action + ACTION_STATE_VALUE_OFFSET) as u64)
                .unwrap_or_default(),
        }
    }

    /// Sets the story action with the id done while it runs (started, not done); the caller is
    /// the game thread. Gives whether the game took the change.
    pub fn set_story_action_done(&self, id: u32, value: i32) -> bool {
        let Some(action) = self.story_action_by_id(id) else {
            return false;
        };
        let running = self
            .read_u32_at((action + ACTION_FLAGS_OFFSET) as u64)
            .is_ok_and(|flags| StepActionState::of_flags(flags) == StepActionState::Running);
        running && self.set_state_by_hash(action, ACTION_DONE_STATE_HASH, value)
    }

    #[expect(
        unsafe_code,
        reason = "calls the story host's find-by-id function of the game"
    )]
    fn story_action_by_id(&self, id: u32) -> Option<usize> {
        let find = self.address_of(FIND_STORY_ACTION_BY_ID_FUNCTION);
        let host = self.address_of(STORY_HOST_OBJECT);
        // SAFETY: the address is the story host's plain find-by-id walk of the verified build
        // (its start was checked at install); it takes the host object and a u32 id, only reads
        // the host's lists and returns an action pointer or 0. The caller is the game thread.
        let action = unsafe { core::mem::transmute::<usize, FindActionById>(find)(host, id) };
        (action != 0).then_some(action)
    }

    #[expect(
        unsafe_code,
        reason = "calls the game's set-state-by-hash wrapper of a story action"
    )]
    fn set_state_by_hash(&self, action: usize, hash: u32, value: i32) -> bool {
        let set = self.address_of(SET_ACTION_STATE_BY_HASH_FUNCTION);
        // SAFETY: the address is the setState wrapper of the verified build (its start was
        // checked at install); it takes a live action found by the game's own lookup, a state
        // hash and a value, and returns whether the state changed. The caller is the game thread.
        unsafe { core::mem::transmute::<usize, SetStateByHash>(set)(action, hash, value) != 0 }
    }
}
