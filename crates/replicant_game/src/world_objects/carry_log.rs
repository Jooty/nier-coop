use std::collections::HashMap;

use crate::world_objects::CARRY_LOG_INTERVAL_MS;

#[derive(Debug, Default)]
pub struct CarryLog {
    last_logged_ms: HashMap<u16, u64>,
}

impl CarryLog {
    pub fn due(&mut self, now_ms: u64, id: u16) -> bool {
        match self.last_logged_ms.get(&id) {
            Some(last) if now_ms.saturating_sub(*last) < CARRY_LOG_INTERVAL_MS => false,
            _ => {
                self.last_logged_ms.insert(id, now_ms);
                true
            }
        }
    }

    pub fn forget(&mut self, id: u16) {
        self.last_logged_ms.remove(&id);
    }

    pub fn clear(&mut self) {
        self.last_logged_ms.clear();
    }
}
