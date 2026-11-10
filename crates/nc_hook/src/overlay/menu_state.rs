use std::collections::HashMap;
use std::time::Instant;

use crate::menu::AreaChangeTracker;

#[derive(Debug)]
pub(super) struct MenuState {
    pub(super) started: Instant,
    pub(super) area_filter: String,
    pub(super) area_changes: AreaChangeTracker,
    pub(super) teleport_message: String,
    pub(super) story_jump_message: String,
    pub(super) fix_message: String,
    pub(super) open_panels: HashMap<u32, bool>,
}

impl MenuState {
    pub(super) fn new() -> Self {
        Self {
            started: Instant::now(),
            area_filter: String::new(),
            area_changes: AreaChangeTracker::default(),
            teleport_message: String::new(),
            story_jump_message: String::new(),
            fix_message: String::new(),
            open_panels: HashMap::new(),
        }
    }

    pub(super) fn now_ms(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}
