use crate::start::{
    ARRIVAL_LIMIT_MS, Candidate, ChapterRecord, HIGHEST_SAVE_SLOT, PHASE_START_RETRY_LIMIT_MS,
    POLL_LOG_INTERVAL_MS, PhaseStartCall, SLOT_LOAD_LIMIT_MS, StartConfig, StartMode, StartPlan,
    StartReadings, TITLE_IDLE_WITHOUT_SUB_PHASE_NAME_MS, TitleIdle, build_plan, candidates,
    has_arrived, hero_mask_has, title_idle,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartState {
    Disabled,
    Refused(String),
    WaitingForTitle,
    ReadingTable,
    PlanReady(Box<StartPlan>),
    PhaseStarting {
        call: PhaseStartCall,
        since_ms: u64,
    },
    SlotLoading {
        since_ms: u64,
    },
    Arriving {
        phase_id: i32,
        phase_name: String,
        since_ms: u64,
    },
    InWorld(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartAction {
    Nothing,
    ReadTable,
    RunPlan(Box<StartPlan>),
    TryPhaseStart(PhaseStartCall),
    LoadSlot(u32),
    EnterSavedScene,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartStep {
    pub action: StartAction,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StartMachine {
    config: StartConfig,
    state: StartState,
    last_poll_log_ms: Option<u64>,
    idle_without_sub_phase_name_since_ms: Option<u64>,
    sub_phase_note_logged: bool,
}

impl StartMachine {
    pub fn new(config: StartConfig, saves_redirected: bool) -> (Self, Vec<String>) {
        let (state, logs) = match (config.mode, saves_redirected) {
            (StartMode::Title, _) => (
                StartState::Disabled,
                vec![
                    "start: title screen chosen, nothing hooked or called for the start".to_owned(),
                ],
            ),
            (_, false) => {
                let line = "start: refused, saves are not redirected".to_owned();
                (StartState::Refused(line.clone()), vec![line])
            }
            (StartMode::Slot, _) if config.slot > HIGHEST_SAVE_SLOT => {
                let line = format!(
                    "start: refused, start_slot {} is not a save slot (0 to {HIGHEST_SAVE_SLOT})",
                    config.slot
                );
                (StartState::Refused(line.clone()), vec![line])
            }
            _ => (StartState::WaitingForTitle, Vec::new()),
        };
        let machine = Self {
            config,
            state,
            last_poll_log_ms: None,
            idle_without_sub_phase_name_since_ms: None,
            sub_phase_note_logged: false,
        };
        (machine, logs)
    }

    pub fn state(&self) -> &StartState {
        &self.state
    }

    pub fn is_past_title(&self) -> bool {
        !matches!(
            self.state,
            StartState::Disabled | StartState::Refused(_) | StartState::WaitingForTitle
        )
    }

    pub fn is_finished(&self) -> bool {
        matches!(
            self.state,
            StartState::Disabled | StartState::Refused(_) | StartState::InWorld(_)
        )
    }

    pub fn is_refused(&self) -> bool {
        matches!(self.state, StartState::Refused(_))
    }

    pub fn is_in_world(&self) -> bool {
        matches!(self.state, StartState::InWorld(_))
    }

    pub fn wants_slot_file_check(&self) -> bool {
        self.config.mode == StartMode::Slot && self.state == StartState::WaitingForTitle
    }

    pub fn is_loading_slot(&self) -> bool {
        matches!(self.state, StartState::SlotLoading { .. })
    }

    pub fn wants_arrival_readings(&self) -> bool {
        matches!(self.state, StartState::Arriving { .. })
    }

    pub fn status_line(&self) -> String {
        match &self.state {
            StartState::Disabled => "start: title screen chosen, not used".to_owned(),
            StartState::Refused(line) => line.clone(),
            StartState::WaitingForTitle => "start: waiting for the title".to_owned(),
            StartState::ReadingTable
            | StartState::PlanReady(_)
            | StartState::PhaseStarting { .. } => "start: calling the chapter start".to_owned(),
            StartState::SlotLoading { .. } => {
                format!("start: loading slot {}", self.config.slot)
            }
            StartState::Arriving { .. } => {
                format!("start: waiting to arrive ({})", self.start_point_label())
            }
            StartState::InWorld(_) => {
                format!("start: in the world ({})", self.start_point_label())
            }
        }
    }

    pub fn step(&mut self, now_ms: u64, readings: &StartReadings) -> StartStep {
        let mut logs = Vec::new();
        let action = match self.state.clone() {
            StartState::Disabled | StartState::Refused(_) | StartState::InWorld(_) => {
                StartAction::Nothing
            }
            StartState::WaitingForTitle => self.step_waiting(now_ms, readings, &mut logs),
            StartState::ReadingTable => StartAction::ReadTable,
            StartState::PlanReady(plan) => {
                self.set_state(StartState::PhaseStarting {
                    call: plan.phase_start.clone(),
                    since_ms: now_ms,
                });
                StartAction::RunPlan(plan)
            }
            StartState::PhaseStarting { call, since_ms } => {
                if now_ms.saturating_sub(since_ms) > PHASE_START_RETRY_LIMIT_MS {
                    self.refuse(
                        "start: refused, the phase start kept returning 0 for 20 s (the story host never became ready)",
                        &mut logs,
                    );
                    StartAction::Nothing
                } else {
                    self.poll_log(
                        now_ms,
                        "start: calling the phase start (a return of 0 means the story host is not ready, so it is retried)".to_owned(),
                        &mut logs,
                    );
                    StartAction::TryPhaseStart(call)
                }
            }
            StartState::SlotLoading { since_ms } => {
                self.step_slot_loading(now_ms, since_ms, readings, &mut logs)
            }
            StartState::Arriving {
                phase_id,
                phase_name,
                since_ms,
            } => self.step_arriving(now_ms, since_ms, phase_id, &phase_name, readings, &mut logs),
        };
        StartStep { action, logs }
    }

    pub fn table_read(&mut self, table: Result<Vec<ChapterRecord>, String>) -> Vec<String> {
        let records = match table {
            Ok(records) => records,
            Err(error) => {
                let mut logs = Vec::new();
                self.refuse(
                    &format!("start: refused, cannot read the chapter table ({error})"),
                    &mut logs,
                );
                return logs;
            }
        };
        let found = candidates(&records);
        let mut logs = Vec::new();
        self.choose(records.len(), &found, &mut logs);
        logs
    }

    pub fn phase_start_returned(&mut self, now_ms: u64, returned_nonzero: bool) -> Vec<String> {
        let mut logs = Vec::new();
        let StartState::PhaseStarting { call, .. } = self.state.clone() else {
            return logs;
        };
        if returned_nonzero {
            logs.push(format!(
                "start: phase start accepted for phase {:#X} {}",
                call.phase_id, call.phase_name
            ));
            self.set_state(StartState::Arriving {
                phase_id: call.phase_id,
                phase_name: call.phase_name,
                since_ms: now_ms,
            });
        }
        logs
    }

    pub fn slot_requested(&mut self, now_ms: u64, busy_right_after: bool) -> Vec<String> {
        let mut logs = Vec::new();
        if busy_right_after {
            self.set_state(StartState::SlotLoading { since_ms: now_ms });
        } else {
            self.refuse(
                &format!(
                    "start: refused, the game did not take the load request for slot {} (the save device is not ready)",
                    self.config.slot
                ),
                &mut logs,
            );
        }
        logs
    }

    pub fn saved_scene_requested(&mut self, now_ms: u64, phase_id: i32, phase_name: String) {
        self.set_state(StartState::Arriving {
            phase_id,
            phase_name,
            since_ms: now_ms,
        });
    }

    pub fn call_failed(&mut self, reason: &str) -> Vec<String> {
        let mut logs = Vec::new();
        self.refuse(&format!("start: refused, {reason}"), &mut logs);
        logs
    }

    fn step_waiting(
        &mut self,
        now_ms: u64,
        readings: &StartReadings,
        logs: &mut Vec<String>,
    ) -> StartAction {
        if !self.title_is_idle_now(now_ms, readings, logs) {
            self.poll_log(
                now_ms,
                format!(
                    "start: waiting for the title: {}",
                    readings.title.describe()
                ),
                logs,
            );
            return StartAction::Nothing;
        }
        logs.push(format!(
            "start: the title is idle ({})",
            readings.title.describe()
        ));
        if self.config.mode == StartMode::Slot {
            return self.begin_slot(readings, logs);
        }
        self.set_state(StartState::ReadingTable);
        StartAction::ReadTable
    }

    fn title_is_idle_now(
        &mut self,
        now_ms: u64,
        readings: &StartReadings,
        logs: &mut Vec<String>,
    ) -> bool {
        match title_idle(&readings.title) {
            TitleIdle::Yes => true,
            TitleIdle::No => {
                self.idle_without_sub_phase_name_since_ms = None;
                false
            }
            TitleIdle::YesButSubPhaseIsNotAName => {
                if !self.sub_phase_note_logged {
                    self.sub_phase_note_logged = true;
                    logs.push(
                        "start: the sub-phase name cannot be read as a name on this exe, so the title counts as idle after 3 s in a row"
                            .to_owned(),
                    );
                }
                let since = *self
                    .idle_without_sub_phase_name_since_ms
                    .get_or_insert(now_ms);
                now_ms.saturating_sub(since) >= TITLE_IDLE_WITHOUT_SUB_PHASE_NAME_MS
            }
        }
    }

    fn begin_slot(&mut self, readings: &StartReadings, logs: &mut Vec<String>) -> StartAction {
        if !readings.slot_calls_verified {
            self.refuse(
                "start: refused, slot start is off on this exe (the bytes at its two inferred save functions are not the expected ones)",
                logs,
            );
            return StartAction::Nothing;
        }
        if !readings.slot_file_present {
            self.refuse(
                &format!(
                    "start: refused, no save for slot {} in the co-op folder",
                    self.config.slot
                ),
                logs,
            );
            return StartAction::Nothing;
        }
        logs.push(format!(
            "start: loading slot {} from the co-op folder",
            self.config.slot
        ));
        StartAction::LoadSlot(self.config.slot)
    }

    fn step_slot_loading(
        &mut self,
        now_ms: u64,
        since_ms: u64,
        readings: &StartReadings,
        logs: &mut Vec<String>,
    ) -> StartAction {
        let title = &readings.title;
        if title.save_failed != Some(false) {
            self.refuse(
                &format!("start: refused, loading slot {} failed", self.config.slot),
                logs,
            );
            return StartAction::Nothing;
        }
        if title.save_busy == Some(false) {
            logs.push("start: the slot is loaded, entering its scene".to_owned());
            return StartAction::EnterSavedScene;
        }
        if now_ms.saturating_sub(since_ms) > SLOT_LOAD_LIMIT_MS {
            self.refuse(
                &format!(
                    "start: refused, slot {} was still loading after 60 s",
                    self.config.slot
                ),
                logs,
            );
            return StartAction::Nothing;
        }
        self.poll_log(
            now_ms,
            format!("start: waiting for the slot to load: {}", title.describe()),
            logs,
        );
        StartAction::Nothing
    }

    fn step_arriving(
        &mut self,
        now_ms: u64,
        since_ms: u64,
        phase_id: i32,
        phase_name: &str,
        readings: &StartReadings,
        logs: &mut Vec<String>,
    ) -> StartAction {
        let Some(arrival) = readings.arrival.as_ref() else {
            return StartAction::Nothing;
        };
        if has_arrived(arrival, phase_id) && arrival.player_held_by.is_some() {
            // No limit here: the scene is reached, and a cutscene may run as long as it likes.
            self.poll_log(
                now_ms,
                format!(
                    "start: reached phase {phase_id:#X}, waiting until the player is free: {}",
                    arrival.describe()
                ),
                logs,
            );
        } else if has_arrived(arrival, phase_id) {
            logs.push(format!(
                "start: in the world, phase {phase_id:#X} {phase_name}"
            ));
            self.set_state(StartState::InWorld(phase_name.to_owned()));
        } else if now_ms.saturating_sub(since_ms) > ARRIVAL_LIMIT_MS {
            self.refuse(
                &format!(
                    "start: refused, phase {phase_id:#X} was not reached after 300 s ({})",
                    arrival.describe()
                ),
                logs,
            );
        } else {
            self.poll_log(
                now_ms,
                format!(
                    "start: waiting to arrive at phase {phase_id:#X}: {}",
                    arrival.describe()
                ),
                logs,
            );
        }
        StartAction::Nothing
    }

    fn choose(&mut self, record_count: usize, found: &[Candidate], logs: &mut Vec<String>) {
        let counts = format!(
            "start: {record_count} chapter records, {} candidates (the candidate rule is a guess)",
            found.len()
        );
        if found.is_empty() {
            logs.push(counts);
            self.refuse(
                "start: refused, no start candidates in the chapter table",
                logs,
            );
            return;
        }
        let Some(chosen) = found.get(self.config.index) else {
            logs.push(counts);
            self.refuse(
                &format!(
                    "start: refused, start_index {} is out of range ({} candidates)",
                    self.config.index,
                    found.len()
                ),
                logs,
            );
            return;
        };
        logs.push(format!(
            "{counts}, chose index {}, phase {:#X}",
            chosen.index, chosen.record.phase_id as u16
        ));
        if !hero_mask_has(chosen.record.hero_mask, self.config.hero) {
            logs.push(format!(
                "start: note, hero mask {:#X} has no bit for {} (the mask meaning is a guess)",
                chosen.record.hero_mask,
                self.config.hero.launcher_id()
            ));
        }
        let plan = build_plan(&chosen.record, self.config.hero, self.config.partner);
        self.set_state(StartState::PlanReady(Box::new(plan)));
    }

    fn start_point_label(&self) -> String {
        if self.config.mode == StartMode::Slot {
            format!("slot {}", self.config.slot)
        } else {
            format!("start point {}", self.config.index)
        }
    }

    fn refuse(&mut self, line: &str, logs: &mut Vec<String>) {
        logs.push(line.to_owned());
        self.set_state(StartState::Refused(line.to_owned()));
    }

    fn set_state(&mut self, state: StartState) {
        self.state = state;
        self.last_poll_log_ms = None;
    }

    fn poll_log(&mut self, now_ms: u64, line: String, logs: &mut Vec<String>) {
        let due = self
            .last_poll_log_ms
            .is_none_or(|last| now_ms.saturating_sub(last) >= POLL_LOG_INTERVAL_MS);
        if due {
            self.last_poll_log_ms = Some(now_ms);
            logs.push(line);
        }
    }
}
