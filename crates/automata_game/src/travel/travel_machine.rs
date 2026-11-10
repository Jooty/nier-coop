use crate::start::{RUNNING_SCENE_STATE, is_in_play};
use crate::travel::{
    ARRIVAL_SHOWN_AS_CURRENT_MS, FAST_TRAVEL_LAST_STEP_BEFORE_THE_JUMP, FOLLOWED_JUMP_WAIT_MS,
    JUMP_ARRIVAL_LIMIT_MS, JumpRefusal, JumpRole, SCENE_LEAVE_LIMIT_MS,
    SCENE_LEAVE_LIMIT_WHILE_THE_TRANSPORT_STEPS_MS, TravelPoint, WorldReadings,
    decide_followed_jump, decide_local_jump,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpOrigin {
    Local,
    Host,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TravelAction {
    Nothing,
    Call {
        point: TravelPoint,
        origin: JumpOrigin,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TravelStep {
    pub action: TravelAction,
    /// The scene left its running state for a jump the local host asked: the guest may follow.
    pub tell_guest: Option<TravelPoint>,
    /// The game's transport still waits at its last step: write the rest value to its counter.
    pub cancel_waiting_step: bool,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Progress {
    #[default]
    Idle,
    Called {
        point: TravelPoint,
        origin: JumpOrigin,
        at_ms: u64,
    },
    SceneChanging {
        point: TravelPoint,
        origin: JumpOrigin,
        since_ms: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WaitingFollow {
    point: TravelPoint,
    waited_in_play_ms: u64,
    last_step_ms: u64,
    last_refusal: Option<JumpRefusal>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Arrival {
    point: TravelPoint,
    at_ms: u64,
}

/// Follows one jump from the call of the game's transport to the scene running again.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TravelMachine {
    progress: Progress,
    waiting_follow: Option<WaitingFollow>,
    arrival: Option<Arrival>,
    line: String,
}

impl TravelStep {
    fn of(action: TravelAction, logs: Vec<String>) -> Self {
        Self {
            action,
            tell_guest: None,
            cancel_waiting_step: false,
            logs,
        }
    }
}

impl TravelMachine {
    pub fn request_local(
        &mut self,
        role: JumpRole,
        world: &WorldReadings,
        point: TravelPoint,
    ) -> TravelStep {
        match decide_local_jump(role, self.is_under_way(), Some(world), point) {
            Ok(()) => {
                self.arrival = None;
                TravelStep::of(
                    TravelAction::Call {
                        point,
                        origin: JumpOrigin::Local,
                    },
                    Vec::new(),
                )
            }
            Err(refusal) => {
                self.line = format!("not jumped to {}: {refusal}", point.label());
                TravelStep::of(
                    TravelAction::Nothing,
                    vec![format!("travel: {}", self.line)],
                )
            }
        }
    }

    pub fn follow_host(&mut self, now_ms: u64, point: TravelPoint) -> Vec<String> {
        // The same jump asked again while it still waits is one wait and one line.
        if self
            .waiting_follow
            .is_some_and(|waiting| waiting.point == point)
        {
            return Vec::new();
        }
        self.waiting_follow = Some(WaitingFollow {
            point,
            waited_in_play_ms: 0,
            last_step_ms: now_ms,
            last_refusal: None,
        });
        self.line = format!("the host jumps to {}; following", point.label());
        vec![format!(
            "travel: the host jumps to {}, this game follows",
            point.code()
        )]
    }

    pub fn call_returned(
        &mut self,
        now_ms: u64,
        point: TravelPoint,
        origin: JumpOrigin,
        taken: bool,
    ) -> Vec<String> {
        if !taken {
            self.line = format!(
                "not jumped to {}: the game's transport did not take the call",
                point.label()
            );
            return vec![format!(
                "travel: the game's transport did not take the call for {}",
                point.code()
            )];
        }
        self.progress = Progress::Called {
            point,
            origin,
            at_ms: now_ms,
        };
        self.line = match origin {
            JumpOrigin::Local => format!("jumping to {}", point.label()),
            JumpOrigin::Host => format!("following the host to {}", point.label()),
        };
        vec![format!(
            "travel: the game's transport took the call for {}; waiting for the scene to leave its running state",
            point.code()
        )]
    }

    pub fn step(&mut self, now_ms: u64, world: &WorldReadings) -> TravelStep {
        let mut logs = Vec::new();
        let mut tell_guest = None;
        let mut cancel_waiting_step = false;
        match self.progress {
            Progress::Idle => {}
            Progress::Called {
                point,
                origin,
                at_ms,
            } => {
                if world.play.scene_state != RUNNING_SCENE_STATE {
                    self.progress = Progress::SceneChanging {
                        point,
                        origin,
                        since_ms: now_ms,
                    };
                    logs.push(format!(
                        "travel: the scene left its running state (state {}), the jump to {} is under way",
                        world.play.scene_state,
                        point.code()
                    ));
                    if origin == JumpOrigin::Local {
                        tell_guest = Some(point);
                    }
                } else if world.transport_left_its_steps() {
                    self.progress = Progress::Idle;
                    self.line = format!(
                        "not jumped to {}: the game's transport dropped the call",
                        point.label()
                    );
                    logs.push(format!(
                        "travel: the transport's step counter reads {:?} and the scene still runs, so no jump to {} will come; the wait ends now",
                        world.fast_travel_step,
                        point.code()
                    ));
                } else if call_is_given_up(now_ms.saturating_sub(at_ms), world) {
                    self.progress = Progress::Idle;
                    self.line = format!(
                        "not jumped to {}: no scene change within {} s of the call",
                        point.label(),
                        now_ms.saturating_sub(at_ms) / 1000
                    );
                    cancel_waiting_step =
                        world.fast_travel_step == Some(FAST_TRAVEL_LAST_STEP_BEFORE_THE_JUMP);
                    logs.push(format!(
                        "travel: no scene change within {} s of the call for {}; the transport's step counter reads {:?}: {}",
                        now_ms.saturating_sub(at_ms) / 1000,
                        point.code(),
                        world.fast_travel_step,
                        if cancel_waiting_step {
                            "it still waits for the point, so the counter is set to its rest value and the game cannot jump later"
                        } else {
                            "it is left alone (below its last step the game's own sequence runs and may still do the jump)"
                        }
                    ));
                }
            }
            Progress::SceneChanging {
                point,
                origin,
                since_ms,
            } => {
                if is_in_play(&world.play) {
                    self.progress = Progress::Idle;
                    self.arrival = Some(Arrival {
                        point,
                        at_ms: now_ms,
                    });
                    self.line = match origin {
                        JumpOrigin::Local => format!("arrived at {}", point.label()),
                        JumpOrigin::Host => format!("followed the host to {}", point.label()),
                    };
                    logs.push(format!(
                        "travel: the scene runs again with a hero, arrived at {}",
                        point.code()
                    ));
                } else if now_ms.saturating_sub(since_ms) >= JUMP_ARRIVAL_LIMIT_MS {
                    self.progress = Progress::Idle;
                    self.line = format!(
                        "the scene did not run again within {} s of the jump to {}",
                        JUMP_ARRIVAL_LIMIT_MS / 1000,
                        point.label()
                    );
                    logs.push(format!("travel: {}", self.line));
                }
            }
        }
        let action = self.try_waiting_follow(now_ms, world, &mut logs);
        TravelStep {
            action,
            tell_guest,
            cancel_waiting_step,
            logs,
        }
    }

    pub fn has_waiting_follow(&self) -> bool {
        self.waiting_follow.is_some()
    }

    pub fn is_under_way(&self) -> bool {
        self.progress != Progress::Idle
    }

    /// True from the taken call until the scene has left its running state: the game asks for
    /// the scene change by itself in that window.
    pub fn expects_scene_request(&self) -> bool {
        matches!(self.progress, Progress::Called { .. })
    }

    pub fn current_point(&self, now_ms: u64) -> Option<TravelPoint> {
        self.arrival
            .filter(|arrival| now_ms.saturating_sub(arrival.at_ms) < ARRIVAL_SHOWN_AS_CURRENT_MS)
            .map(|arrival| arrival.point)
    }

    pub fn line(&self) -> &str {
        &self.line
    }

    fn try_waiting_follow(
        &mut self,
        now_ms: u64,
        world: &WorldReadings,
        logs: &mut Vec<String>,
    ) -> TravelAction {
        let Some(waiting) = self.waiting_follow else {
            return TravelAction::Nothing;
        };
        let point = waiting.point;
        // The wait counts only while this game could follow: a load of its own does not use it up.
        let counts = is_in_play(&world.play) && !self.is_under_way();
        let waited_in_play_ms = waiting.waited_in_play_ms
            + if counts {
                now_ms.saturating_sub(waiting.last_step_ms)
            } else {
                0
            };
        let waiting = WaitingFollow {
            waited_in_play_ms,
            last_step_ms: now_ms,
            ..waiting
        };
        self.waiting_follow = Some(waiting);
        match decide_followed_jump(self.is_under_way(), Some(world), point) {
            Ok(()) => {
                self.waiting_follow = None;
                self.arrival = None;
                TravelAction::Call {
                    point,
                    origin: JumpOrigin::Host,
                }
            }
            Err(refusal) => {
                if waited_in_play_ms >= FOLLOWED_JUMP_WAIT_MS {
                    self.waiting_follow = None;
                    self.line = format!("did not follow the host to {}: {refusal}", point.label());
                    logs.push(format!("travel: {}", self.line));
                } else if waiting.last_refusal != Some(refusal) {
                    self.waiting_follow = Some(WaitingFollow {
                        last_refusal: Some(refusal),
                        ..waiting
                    });
                    logs.push(format!(
                        "travel: cannot follow the host to {} yet ({refusal}); trying for up to {} s in play",
                        point.code(),
                        FOLLOWED_JUMP_WAIT_MS / 1000
                    ));
                }
                TravelAction::Nothing
            }
        }
    }
}

fn call_is_given_up(waited_ms: u64, world: &WorldReadings) -> bool {
    let limit_ms = if world.transport_is_stepping() {
        SCENE_LEAVE_LIMIT_WHILE_THE_TRANSPORT_STEPS_MS
    } else {
        SCENE_LEAVE_LIMIT_MS
    };
    waited_ms >= limit_ms
}
