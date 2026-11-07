use crate::scene_follow::SCENE_FOLLOW_DUPLICATE_WINDOW_MS;

#[derive(Debug, Clone, Default)]
pub struct InjectedCallbacks {
    entries: Vec<InjectedEntry>,
}

#[derive(Debug, Clone)]
struct InjectedEntry {
    name: String,
    at_ms: u64,
    popped: bool,
}

impl InjectedCallbacks {
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn add(&mut self, name: &str, now_ms: u64) {
        self.entries.retain(|entry| entry.name != name);
        self.entries.push(InjectedEntry {
            name: name.to_owned(),
            at_ms: now_ms,
            popped: false,
        });
    }

    pub fn is_duplicate(&mut self, name: &str, now_ms: u64) -> bool {
        self.entries
            .retain(|entry| now_ms.saturating_sub(entry.at_ms) <= SCENE_FOLLOW_DUPLICATE_WINDOW_MS);
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.name == name) else {
            return false;
        };
        let duplicate = entry.popped;
        entry.popped = true;
        duplicate
    }
}
