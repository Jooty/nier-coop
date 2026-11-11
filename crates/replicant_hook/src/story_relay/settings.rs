use std::sync::atomic::{AtomicBool, Ordering};

static SHARED_STORY_EVENTS: AtomicBool = AtomicBool::new(true);

pub fn shared_story_events() -> bool {
    SHARED_STORY_EVENTS.load(Ordering::Relaxed)
}

pub fn set_shared_story_events(enabled: bool) {
    SHARED_STORY_EVENTS.store(enabled, Ordering::Relaxed);
}
