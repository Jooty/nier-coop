use crate::character::Character;
use crate::guest_watches::{
    HAND_BACK_LOAD_MAY_TAKE_MS, WATCHING_LINE_INTERVAL_MS, WatchedStretch,
    watching_ended_by_the_bound_line, watching_ended_by_the_load_line,
    watching_ended_without_the_host_line, watching_goes_on_outside_line, watching_started_line,
};
use crate::load_gate::StoryPlace;

/// What the guest's machine reads each tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchReadings {
    pub now_ms: u64,
    /// The setting is on, this copy is the campaign guest and its host is present.
    pub applies: bool,
    pub place: StoryPlace,
    pub in_play: bool,
    pub hero: Option<Character>,
    pub host: Option<Character>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WatchStep {
    pub watching: bool,
    pub started: bool,
    pub ended: bool,
    pub logs: Vec<String>,
}

/// What a full load the guest's game is about to make means for watching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullLoadVerdict {
    NotWatching,
    StaysWatching,
    /// Write the guest's own character as the hero before the load; watching ends with it.
    HandsBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Watch {
    #[default]
    Idle,
    Watching {
        outside_logged_at_ms: Option<u64>,
    },
    HandingBack {
        since_ms: u64,
        out_of_play_seen: bool,
    },
}

/// The guest's side of watching: when it starts, how it goes on, and the hand-back at a full load.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuestWatch {
    stretch: WatchedStretch,
    watch: Watch,
}

impl GuestWatch {
    pub fn new(stretch: WatchedStretch) -> Self {
        Self {
            stretch,
            watch: Watch::Idle,
        }
    }

    pub fn stretch(&self) -> &WatchedStretch {
        &self.stretch
    }

    /// True from the start of watching until the load of the hand-back has ended.
    pub fn is_watching(&self) -> bool {
        self.watch != Watch::Idle
    }

    pub fn step(&mut self, readings: &WatchReadings) -> WatchStep {
        let mut step = WatchStep::default();
        match self.watch {
            Watch::Idle => {
                if readings.applies && self.stretch.holds(&readings.place) {
                    self.watch = Watch::Watching {
                        outside_logged_at_ms: None,
                    };
                    step.started = true;
                    step.logs.push(watching_started_line(
                        &readings.place,
                        readings.hero,
                        readings.host,
                    ));
                }
            }
            Watch::Watching { .. } | Watch::HandingBack { .. } if !readings.applies => {
                self.end(
                    &mut step,
                    watching_ended_without_the_host_line(&readings.place),
                );
            }
            Watch::Watching {
                outside_logged_at_ms,
            } => self.go_on_watching(readings, outside_logged_at_ms, &mut step),
            Watch::HandingBack {
                since_ms,
                out_of_play_seen,
            } => {
                let out_of_play_seen = out_of_play_seen || !readings.in_play;
                let waited_ms = readings.now_ms.saturating_sub(since_ms);
                if out_of_play_seen && readings.in_play {
                    self.end(&mut step, watching_ended_by_the_load_line(&readings.place));
                } else if waited_ms > HAND_BACK_LOAD_MAY_TAKE_MS {
                    self.end(
                        &mut step,
                        watching_ended_by_the_bound_line(&readings.place, waited_ms),
                    );
                } else {
                    self.watch = Watch::HandingBack {
                        since_ms,
                        out_of_play_seen,
                    };
                }
            }
        }
        step.watching = self.is_watching();
        step
    }

    /// Called just before the guest's game makes a full load to `target`.
    pub fn before_a_full_load(&mut self, now_ms: u64, target: &StoryPlace) -> FullLoadVerdict {
        match self.watch {
            Watch::Idle => FullLoadVerdict::NotWatching,
            Watch::Watching { .. } | Watch::HandingBack { .. }
                if self.stretch.a_full_load_there_ends_watching(target) =>
            {
                self.watch = Watch::HandingBack {
                    since_ms: now_ms,
                    out_of_play_seen: false,
                };
                FullLoadVerdict::HandsBack
            }
            Watch::Watching { .. } | Watch::HandingBack { .. } => FullLoadVerdict::StaysWatching,
        }
    }

    fn go_on_watching(
        &mut self,
        readings: &WatchReadings,
        outside_logged_at_ms: Option<u64>,
        step: &mut WatchStep,
    ) {
        let outside = readings.place.has_a_name() && !self.stretch.holds(&readings.place);
        let due = outside_logged_at_ms.is_none_or(|logged| {
            readings.now_ms.saturating_sub(logged) >= WATCHING_LINE_INTERVAL_MS
        });
        let outside_logged_at_ms = if outside && due {
            step.logs
                .push(watching_goes_on_outside_line(&readings.place));
            Some(readings.now_ms)
        } else if outside {
            outside_logged_at_ms
        } else {
            None
        };
        self.watch = Watch::Watching {
            outside_logged_at_ms,
        };
    }

    fn end(&mut self, step: &mut WatchStep, line: String) {
        self.watch = Watch::Idle;
        step.ended = true;
        step.logs.push(line);
    }
}
