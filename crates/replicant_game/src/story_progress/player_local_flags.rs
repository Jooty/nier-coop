//! Game flags each game keeps for its own player (the side-view state its area traps track).

use replicant_proto::{StoryArray, StoryChange};

use crate::fight_sync::TRAP_PRIVATE_GAME_FLAGS;
use crate::story_progress::{StoryArrays, StoryKey};

/// A game flag that only area-trap callbacks read and write: each game's own, never sent or applied.
pub fn is_player_local_flag(key: StoryKey) -> bool {
    key.array == StoryArray::GameFlags && TRAP_PRIVATE_GAME_FLAGS.contains(&key.id)
}

/// `arrays` with every player-local flag cleared, so a copy sent or compared never carries them.
pub fn without_player_local_flags(arrays: &StoryArrays) -> StoryArrays {
    let mut shared = arrays.clone();
    for &id in &TRAP_PRIVATE_GAME_FLAGS {
        shared.apply(StoryChange::Flag {
            array: StoryArray::GameFlags,
            id,
            on: false,
        });
    }
    shared
}
