use std::sync::Mutex;

use automata_game::load_gate::{STORY_PLACE_NAME_MOST_BYTES, StoryPlace};

static LOCAL_STORY_PLACE: Mutex<StoryPlace> = Mutex::new(StoryPlace {
    phase_id: 0,
    name_len: 0,
    name_bytes: [0; STORY_PLACE_NAME_MOST_BYTES],
});

/// What every state message of this game says about the place it is in from now on.
pub fn set_local_story_place(place: StoryPlace) {
    if let Ok(mut held) = LOCAL_STORY_PLACE.lock() {
        *held = place;
    }
}

pub fn local_story_place() -> StoryPlace {
    LOCAL_STORY_PLACE
        .lock()
        .map(|held| *held)
        .unwrap_or_default()
}
