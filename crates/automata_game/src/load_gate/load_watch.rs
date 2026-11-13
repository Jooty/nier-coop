use crate::load_gate::FIRST_SYSTEM_PHASE_ID;
use crate::start::RUNNING_SCENE_STATE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadWatchReadings {
    pub scene_state: Option<i32>,
    pub phase_id: Option<i32>,
    pub campaign_link_up: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LoadWatchStep {
    pub began: bool,
    pub ended: bool,
    pub link_lost: bool,
    pub phase_is_a_system_phase: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchState {
    Idle { scene_seen_running: bool },
    Loading,
}

/// Says when a full scene load of a campaign session begins and when it has ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadWatch {
    state: WatchState,
}

impl Default for LoadWatch {
    fn default() -> Self {
        Self {
            state: WatchState::Idle {
                scene_seen_running: false,
            },
        }
    }
}

impl LoadWatch {
    pub fn is_loading(&self) -> bool {
        self.state == WatchState::Loading
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn step(&mut self, readings: &LoadWatchReadings) -> LoadWatchStep {
        let running = readings
            .scene_state
            .map(|scene_state| scene_state == RUNNING_SCENE_STATE);
        match self.state {
            WatchState::Idle { scene_seen_running } => {
                let began = scene_seen_running && running == Some(false);
                if began && readings.campaign_link_up {
                    self.state = WatchState::Loading;
                    return LoadWatchStep {
                        began: true,
                        phase_is_a_system_phase: is_a_system_phase(readings.phase_id),
                        ..LoadWatchStep::default()
                    };
                }
                if let Some(running) = running {
                    self.state = WatchState::Idle {
                        scene_seen_running: running,
                    };
                }
                LoadWatchStep::default()
            }
            WatchState::Loading => {
                let link_lost = !readings.campaign_link_up;
                let ended = link_lost || running == Some(true);
                if ended {
                    self.state = WatchState::Idle {
                        scene_seen_running: running == Some(true),
                    };
                }
                LoadWatchStep {
                    ended,
                    link_lost,
                    phase_is_a_system_phase: is_a_system_phase(readings.phase_id),
                    ..LoadWatchStep::default()
                }
            }
        }
    }
}

fn is_a_system_phase(phase_id: Option<i32>) -> bool {
    phase_id.is_some_and(|phase_id| phase_id >= FIRST_SYSTEM_PHASE_ID)
}
