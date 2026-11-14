use crate::own_ride::{DROPPED_RIDE_STEP_SETTLES_AFTER_MS, DROPPED_RIDE_STEP_WATCHED_FOR_MS};

/// What this game shows when it may take a dropped boarding step's pause off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DroppedStepView {
    /// Control mask slot 0, where the script pause (`_PauseOn`) sits.
    pub slot_zero: u32,
    pub script_slot_idle: bool,
    pub partner_calls_clear: bool,
    /// A talk window, a menu or prompt, or an event scene is up.
    pub something_open: bool,
    pub in_play: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DroppedStepRelease {
    pub step: String,
    pub mask: u32,
    pub after_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Dropped {
    step: String,
    at_ms: u64,
    last_step_ms: u64,
}

/// The partner's boarding step was dropped here because the partner rides. The steps before it
/// paused this game for the ride, and only the ride's end (which never runs here) takes that
/// pause off, so once the partner's steps settle this game is let go.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DroppedRideStep {
    dropped: Option<Dropped>,
}

impl DroppedRideStep {
    pub const fn new() -> Self {
        Self { dropped: None }
    }

    pub fn note_dropped(&mut self, step: &str, now_ms: u64) {
        self.dropped = Some(Dropped {
            step: step.to_owned(),
            at_ms: now_ms,
            last_step_ms: now_ms,
        });
    }

    /// A partner step arrived, or a step ran here: the release waits for the steps to settle.
    pub fn note_step(&mut self, now_ms: u64) {
        if let Some(dropped) = self.dropped.as_mut() {
            dropped.last_step_ms = dropped.last_step_ms.max(now_ms);
        }
    }

    /// This game boards or rides itself: its own ride's end takes the pause off.
    pub fn forget(&mut self) {
        self.dropped = None;
    }

    pub const fn is_watching(&self) -> bool {
        self.dropped.is_some()
    }

    pub fn observe(&mut self, view: DroppedStepView, now_ms: u64) -> Option<DroppedStepRelease> {
        let dropped = self.dropped.as_ref()?;
        let since_drop_ms = now_ms.saturating_sub(dropped.at_ms);
        if view.slot_zero == 0 || since_drop_ms > DROPPED_RIDE_STEP_WATCHED_FOR_MS {
            self.dropped = None;
            return None;
        }
        let settled =
            now_ms.saturating_sub(dropped.last_step_ms) >= DROPPED_RIDE_STEP_SETTLES_AFTER_MS;
        let free = view.script_slot_idle
            && view.partner_calls_clear
            && !view.something_open
            && view.in_play;
        if !settled || !free {
            return None;
        }
        let step = self.dropped.take()?.step;
        Some(DroppedStepRelease {
            step,
            mask: view.slot_zero,
            after_ms: since_drop_ms,
        })
    }
}
