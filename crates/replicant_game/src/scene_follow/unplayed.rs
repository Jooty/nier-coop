use crate::callback_gate::{GATED_EVENT_SUFFIX, event_base, is_end_callback};
use crate::scene_follow::UNPLAYED_END_AFTER_START_MAX_MS;

/// Spots the end a scene start gets when its cut was never loaded: the engine reveals the event
/// layer and queues the end at once, so that end pops with no cut loaded, right after its start.
#[derive(Debug, Clone, Default)]
pub struct UnplayedStarts {
    last_start: Option<(String, u64)>,
    no_cut_reveal_ms: Option<u64>,
}

impl UnplayedStarts {
    pub const fn new() -> Self {
        Self {
            last_start: None,
            no_cut_reveal_ms: None,
        }
    }

    pub fn note_popped(&mut self, name: &str, now_ms: u64) {
        if name.ends_with(GATED_EVENT_SUFFIX) {
            self.last_start = Some((name.to_owned(), now_ms));
            self.no_cut_reveal_ms = None;
        }
    }

    pub fn note_no_cut_reveal(&mut self, now_ms: u64) {
        self.no_cut_reveal_ms = Some(now_ms);
    }

    /// The start whose scene never played, when `name` is its end popping with no cut loaded.
    pub fn take_unplayed_start(
        &mut self,
        name: &str,
        cut_loaded: bool,
        now_ms: u64,
    ) -> Option<String> {
        if cut_loaded || !is_end_callback(name) {
            return None;
        }
        let (start, started_ms) = self.last_start.as_ref()?;
        let same_scene = event_base(start) == event_base(name);
        let soon = now_ms.saturating_sub(*started_ms) <= UNPLAYED_END_AFTER_START_MAX_MS;
        let revealed = self
            .no_cut_reveal_ms
            .is_some_and(|revealed| revealed >= *started_ms && revealed <= now_ms);
        if !(same_scene && soon && revealed) {
            return None;
        }
        self.no_cut_reveal_ms = None;
        self.last_start.take().map(|(start, _)| start)
    }
}
