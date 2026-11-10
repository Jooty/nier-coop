use std::fmt;

use crate::load_gate::{
    HIGHEST_ROUND_TRIP_ADDED_TO_THE_RELEASE_LEAD_MS, HoldReading,
    LONGEST_WAIT_FOR_A_NAMED_RELEASE_MS, LoadSignal, PARTNER_READY_WAIT_LIMIT_MS, RELEASE_LEAD_MS,
};
use crate::travel::JumpRole;

/// What the link says about the other player's load right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PartnerLoad {
    pub waiting: bool,
    pub in_play: bool,
    pub release_at_unix_ms: Option<u64>,
    pub round_trip_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GateInputs {
    /// A clock of this game only; it never jumps.
    pub now_ms: u64,
    /// The wall clock, the one both games share.
    pub now_unix_ms: u64,
    pub role: JumpRole,
    /// The local loading screen has nothing left to show and goes away as soon as it is asked to.
    pub loading_screen_ready: bool,
    pub partner: PartnerLoad,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AloneReason {
    NoPartnerLinked,
    PartnerNotReadyInTime,
    PartnerAlreadyInPlay,
    HoldEndedByTheGame,
    HoldEndedByTheHardLimit,
    LoadEndedUnheld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Release {
    Together {
        at_unix_ms: u64,
    },
    Alone {
        reason: AloneReason,
        at_unix_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateState {
    Off(String),
    Loading,
    Waiting { since_ms: u64, screen_ready: bool },
    ReleaseNamed { at_unix_ms: u64, heard_at_ms: u64 },
    Released(Release),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GateStep {
    /// The held end of the load is let go now.
    pub release: bool,
    pub logs: Vec<String>,
}

/// Keeps a finished load on its loading screen until both players' loads are done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadGate {
    state: GateState,
}

impl fmt::Display for AloneReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoPartnerLinked => "no partner was linked in time",
            Self::PartnerNotReadyInTime => "the partner's load was not done in time",
            Self::PartnerAlreadyInPlay => "the partner is already in the world",
            Self::HoldEndedByTheGame => "the game ended the load although it was held",
            Self::HoldEndedByTheHardLimit => "the hold reached its hard limit",
            Self::LoadEndedUnheld => "the start was over before the end of its load could be held",
        })
    }
}

impl LoadGate {
    pub fn new(
        start_together: bool,
        in_a_session: bool,
        hold_installed: bool,
    ) -> (Self, Vec<String>) {
        let off = if !start_together {
            Some("the setting start_together is off")
        } else if !in_a_session {
            Some("there is no session")
        } else if !hold_installed {
            Some("the end of the load cannot be held on this exe")
        } else {
            None
        };
        match off {
            Some(reason) => {
                let line = format!(
                    "start: the loading screen is not kept for the partner ({reason}), the load ends as the game ends it"
                );
                (
                    Self {
                        state: GateState::Off(reason.to_owned()),
                    },
                    vec![line],
                )
            }
            None => (
                Self {
                    state: GateState::Loading,
                },
                vec![format!(
                    "start: the loading screen is kept until both loads are done, at most {} s",
                    PARTNER_READY_WAIT_LIMIT_MS / 1000
                )],
            ),
        }
    }

    pub fn state(&self) -> &GateState {
        &self.state
    }

    /// True while the end of a load is to be caught or kept.
    pub fn wants_hold(&self) -> bool {
        matches!(
            self.state,
            GateState::Loading | GateState::Waiting { .. } | GateState::ReleaseNamed { .. }
        )
    }

    pub fn is_holding(&self) -> bool {
        matches!(
            self.state,
            GateState::Waiting { .. } | GateState::ReleaseNamed { .. }
        )
    }

    pub fn release(&self) -> Option<Release> {
        match self.state {
            GateState::Released(release) => Some(release),
            _ => None,
        }
    }

    /// What this game tells the other one; the sender marks a message without a body itself.
    pub fn signal(&self) -> LoadSignal {
        let release_at_unix_ms = match self.state {
            GateState::ReleaseNamed { at_unix_ms, .. }
            | GateState::Released(
                Release::Together { at_unix_ms } | Release::Alone { at_unix_ms, .. },
            ) => Some(at_unix_ms),
            _ => None,
        };
        LoadSignal {
            without_body: false,
            waiting: matches!(
                self.state,
                GateState::Waiting {
                    screen_ready: true,
                    ..
                } | GateState::ReleaseNamed { .. }
            ),
            release_at_unix_ms,
        }
    }

    /// The status line while the loading screen is kept.
    pub fn holding_status(&self, now_ms: u64) -> Option<String> {
        match &self.state {
            GateState::Waiting { since_ms, .. } => Some(format!(
                "start: load done, waiting for the partner (at most {} s more)",
                PARTNER_READY_WAIT_LIMIT_MS.saturating_sub(now_ms.saturating_sub(*since_ms)) / 1000
            )),
            GateState::ReleaseNamed { .. } => {
                Some("start: both loads are done, leaving the loading screen together".to_owned())
            }
            _ => None,
        }
    }

    /// What the status line says for the rest of the session after a start without the partner.
    pub fn alone_note(&self) -> Option<String> {
        match &self.state {
            GateState::Released(Release::Alone { reason, .. }) => {
                Some(format!("went on alone, {reason}"))
            }
            _ => None,
        }
    }

    /// The hook caught the end of the load and keeps it.
    pub fn load_done(&mut self, now_ms: u64) -> Vec<String> {
        if self.state != GateState::Loading {
            return Vec::new();
        }
        self.state = GateState::Waiting {
            since_ms: now_ms,
            screen_ready: false,
        };
        vec![format!(
            "start: load done, waiting for the partner (at most {} s)",
            PARTNER_READY_WAIT_LIMIT_MS / 1000
        )]
    }

    /// The hold is over without this gate having let it go.
    pub fn hold_ended(&mut self, reading: HoldReading, now_unix_ms: u64) -> Vec<String> {
        let reason = match reading {
            HoldReading::EndedByTheGame => AloneReason::HoldEndedByTheGame,
            HoldReading::EndedByTheHardLimit => AloneReason::HoldEndedByTheHardLimit,
            HoldReading::NotHeld | HoldReading::Held | HoldReading::Released => return Vec::new(),
        };
        if !self.wants_hold() {
            return Vec::new();
        }
        let mut logs = Vec::new();
        self.go_alone(reason, now_unix_ms, &mut logs);
        logs
    }

    /// The start this gate served is over; a hold still wanted is given up.
    pub fn start_is_over(&mut self, now_unix_ms: u64) -> Vec<String> {
        if !self.wants_hold() {
            return Vec::new();
        }
        let mut logs = Vec::new();
        self.go_alone(AloneReason::LoadEndedUnheld, now_unix_ms, &mut logs);
        logs
    }

    /// A gate for a game the hook does not start: it never holds and says nothing.
    pub fn unused() -> Self {
        Self {
            state: GateState::Off("the hook starts nothing".to_owned()),
        }
    }

    pub fn step(&mut self, inputs: &GateInputs) -> GateStep {
        let mut logs = Vec::new();
        let release = match self.state {
            GateState::Off(_) | GateState::Loading | GateState::Released(_) => false,
            GateState::Waiting {
                since_ms,
                screen_ready,
            } => self.step_waiting(since_ms, screen_ready, inputs, &mut logs),
            GateState::ReleaseNamed {
                at_unix_ms,
                heard_at_ms,
            } => self.step_named(at_unix_ms, heard_at_ms, inputs, &mut logs),
        };
        GateStep { release, logs }
    }

    fn step_waiting(
        &mut self,
        since_ms: u64,
        was_ready: bool,
        inputs: &GateInputs,
        logs: &mut Vec<String>,
    ) -> bool {
        let partner = inputs.partner;
        // Ready stays ready: the partner is told once and may already be naming a time.
        let screen_ready = was_ready || inputs.loading_screen_ready;
        if screen_ready && !was_ready {
            logs.push(
                "start: the loading screen has nothing left to show, the partner is told this game is ready"
                    .to_owned(),
            );
            self.state = GateState::Waiting {
                since_ms,
                screen_ready,
            };
        }
        if let Some(at_unix_ms) = partner.release_at_unix_ms {
            logs.push(format!(
                "start: the partner named the release time {at_unix_ms} ({})",
                moment_from_now(at_unix_ms, inputs.now_unix_ms)
            ));
            self.state = GateState::ReleaseNamed {
                at_unix_ms,
                heard_at_ms: inputs.now_ms,
            };
            return self.step_named(at_unix_ms, inputs.now_ms, inputs, logs);
        }
        if partner.in_play {
            self.go_alone(AloneReason::PartnerAlreadyInPlay, inputs.now_unix_ms, logs);
            return true;
        }
        if inputs.role == JumpRole::Host && screen_ready && partner.waiting {
            let lead_ms = RELEASE_LEAD_MS
                + partner
                    .round_trip_ms
                    .unwrap_or(0)
                    .min(HIGHEST_ROUND_TRIP_ADDED_TO_THE_RELEASE_LEAD_MS);
            let at_unix_ms = inputs.now_unix_ms + lead_ms;
            logs.push(format!(
                "start: both loads are done, the host names the release time {at_unix_ms} (in {lead_ms} ms)"
            ));
            self.state = GateState::ReleaseNamed {
                at_unix_ms,
                heard_at_ms: inputs.now_ms,
            };
            return false;
        }
        if inputs.now_ms.saturating_sub(since_ms) >= PARTNER_READY_WAIT_LIMIT_MS {
            let reason = if inputs.role == JumpRole::Solo {
                AloneReason::NoPartnerLinked
            } else {
                AloneReason::PartnerNotReadyInTime
            };
            self.go_alone(reason, inputs.now_unix_ms, logs);
            return true;
        }
        false
    }

    fn step_named(
        &mut self,
        at_unix_ms: u64,
        heard_at_ms: u64,
        inputs: &GateInputs,
        logs: &mut Vec<String>,
    ) -> bool {
        let due = inputs.now_unix_ms >= at_unix_ms;
        let waited_out =
            inputs.now_ms.saturating_sub(heard_at_ms) >= LONGEST_WAIT_FOR_A_NAMED_RELEASE_MS;
        if !due && !waited_out {
            return false;
        }
        if !due {
            logs.push(format!(
                "start: the release time is still ahead after {} s, the two PCs' clocks differ; not waiting longer",
                LONGEST_WAIT_FOR_A_NAMED_RELEASE_MS / 1000
            ));
        }
        logs.push(format!(
            "start: leaving the loading screen together, release time {at_unix_ms} ({})",
            moment_from_now(at_unix_ms, inputs.now_unix_ms)
        ));
        self.state = GateState::Released(Release::Together { at_unix_ms });
        true
    }

    fn go_alone(&mut self, reason: AloneReason, now_unix_ms: u64, logs: &mut Vec<String>) {
        logs.push(format!("start: went on alone, {reason}"));
        self.state = GateState::Released(Release::Alone {
            reason,
            at_unix_ms: now_unix_ms,
        });
    }
}

fn moment_from_now(at_unix_ms: u64, now_unix_ms: u64) -> String {
    if at_unix_ms > now_unix_ms {
        format!("in {} ms", at_unix_ms - now_unix_ms)
    } else {
        format!("{} ms ago", now_unix_ms - at_unix_ms)
    }
}
