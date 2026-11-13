use crate::start::{
    NEW_GAME_OPENING_PHASE_NAME, NEW_GAME_SCENE_REQUEST_PARAMETERS, OPENING_MISSION_PHASE_ID,
    SAVED_SCENE_REQUEST_PARAMETERS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SavedSceneRequest {
    pub parameters: [i32; 6],
    pub is_the_opening: bool,
}

/// What the title's own load flow passes for a loaded slot: a save made at the very start of a new
/// game is entered the way a new game enters it.
pub fn saved_scene_request(phase_id: i32, phase_name: &str) -> SavedSceneRequest {
    let is_the_opening = phase_id == i32::from(OPENING_MISSION_PHASE_ID)
        && phase_name == NEW_GAME_OPENING_PHASE_NAME;
    SavedSceneRequest {
        parameters: if is_the_opening {
            NEW_GAME_SCENE_REQUEST_PARAMETERS
        } else {
            SAVED_SCENE_REQUEST_PARAMETERS
        },
        is_the_opening,
    }
}
