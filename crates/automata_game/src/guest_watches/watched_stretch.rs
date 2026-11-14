use crate::load_gate::StoryPlace;

pub const WATCHED_PHASE_ID: i32 = 0x100;
pub const WATCHED_SUB_PHASE_MARK: &str = "RobotM_Pro";

/// The places where the guest watches the host play: the one table of the rule.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WatchedStretch {
    ends_at: Option<String>,
}

impl WatchedStretch {
    /// `ends_at` is the sub-phase where watching ends instead of the first one without the mark;
    /// an empty or blank text means none.
    pub fn new(ends_at: Option<&str>) -> Self {
        Self {
            ends_at: ends_at
                .map(str::trim)
                .filter(|end| !end.is_empty())
                .map(str::to_owned),
        }
    }

    pub fn ends_at(&self) -> Option<&str> {
        self.ends_at.as_deref()
    }

    /// Watching starts, and goes on, in these places.
    pub fn holds(&self, place: &StoryPlace) -> bool {
        let name = place.sub_phase();
        place.phase_id == WATCHED_PHASE_ID
            && name.contains(WATCHED_SUB_PHASE_MARK)
            && self.ends_at() != Some(name.as_str())
    }

    /// True when a full load to `target` is where watching ends.
    pub fn a_full_load_there_ends_watching(&self, target: &StoryPlace) -> bool {
        match self.ends_at() {
            Some(end) => target.phase_id != WATCHED_PHASE_ID || target.sub_phase() == end,
            None => !self.holds(target),
        }
    }

    pub fn describe(&self) -> String {
        match self.ends_at() {
            Some(end) => format!(
                "phase {WATCHED_PHASE_ID:#X}, from a sub-phase whose name holds {WATCHED_SUB_PHASE_MARK} up to {end} (moved by the setting)"
            ),
            None => format!(
                "phase {WATCHED_PHASE_ID:#X}, the sub-phases whose name holds {WATCHED_SUB_PHASE_MARK}"
            ),
        }
    }
}
