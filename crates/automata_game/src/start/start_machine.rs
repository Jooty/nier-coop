use crate::start::{
    Candidate, ChapterRecord, PHASE_START_RETRY_LIMIT_MS, POLL_LOG_INTERVAL_MS, PhaseStartCall,
    SAVE_BUSY_SETTLE_MS, StartConfig, StartMode, StartPlan, StartReadings, build_plan, candidates,
    has_arrived, hero_mask_has, table_log_lines, title_is_idle,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartState {
    Disabled,
    Refused(String),
    WaitingForTitle,
    ReadingTable,
    PlanReady(Box<StartPlan>),
    PhaseStarting { call: PhaseStartCall, since_ms: u64 },
    SlotLoading { since_ms: u64, saw_busy: bool },
    Arriving { phase_id: i32, phase_name: String },
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
            _ => (StartState::WaitingForTitle, Vec::new()),
        };
        let machine = Self {
            config,
            state,
            last_poll_log_ms: None,
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
            StartState::Arriving { phase_id, .. } => {
                format!("start: waiting to arrive (phase {phase_id:#X})")
            }
            StartState::InWorld(name) => format!("start: in the world ({name})"),
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
            StartState::SlotLoading { since_ms, saw_busy } => {
                self.step_slot_loading(now_ms, since_ms, saw_busy, readings, &mut logs)
            }
            StartState::Arriving {
                phase_id,
                phase_name,
            } => self.step_arriving(now_ms, phase_id, &phase_name, readings, &mut logs),
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
        let mut logs = table_log_lines(&records, &found);
        logs.push(format!(
            "start: {} chapter records, {} candidates (the candidate rule and the base and opening names are guesses)",
            records.len(),
            found.len()
        ));
        self.choose(&found, &mut logs);
        logs
    }

    pub fn phase_start_returned(&mut self, returned_nonzero: bool) -> Vec<String> {
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
            });
        }
        logs
    }

    pub fn slot_requested(&mut self, now_ms: u64) {
        self.set_state(StartState::SlotLoading {
            since_ms: now_ms,
            saw_busy: false,
        });
    }

    pub fn saved_scene_requested(&mut self, phase_id: i32, phase_name: String) {
        self.set_state(StartState::Arriving {
            phase_id,
            phase_name,
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
        if !title_is_idle(&readings.title) {
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
            "start: the title is idle ({}); scene state 3 is a guess until this line appears",
            readings.title.describe()
        ));
        if self.config.mode == StartMode::Slot {
            return self.begin_slot(readings, logs);
        }
        self.set_state(StartState::ReadingTable);
        StartAction::ReadTable
    }

    fn begin_slot(&mut self, readings: &StartReadings, logs: &mut Vec<String>) -> StartAction {
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
        saw_busy: bool,
        readings: &StartReadings,
        logs: &mut Vec<String>,
    ) -> StartAction {
        let title = &readings.title;
        if title.save_failed == Some(true) {
            self.refuse(
                &format!("start: refused, loading slot {} failed", self.config.slot),
                logs,
            );
            return StartAction::Nothing;
        }
        let busy_now = title.save_busy == Some(true);
        let saw_busy = saw_busy || busy_now;
        let settled = now_ms.saturating_sub(since_ms) > SAVE_BUSY_SETTLE_MS;
        if !busy_now && (saw_busy || settled) {
            logs.push("start: the slot is loaded, entering its scene".to_owned());
            return StartAction::EnterSavedScene;
        }
        self.state = StartState::SlotLoading { since_ms, saw_busy };
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
        phase_id: i32,
        phase_name: &str,
        readings: &StartReadings,
        logs: &mut Vec<String>,
    ) -> StartAction {
        let Some(arrival) = readings.arrival.as_ref() else {
            return StartAction::Nothing;
        };
        if has_arrived(arrival, phase_id, phase_name) {
            logs.push(format!(
                "start: in the world, phase {phase_id:#X} {phase_name}"
            ));
            self.set_state(StartState::InWorld(phase_name.to_owned()));
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

    fn choose(&mut self, found: &[Candidate], logs: &mut Vec<String>) {
        if found.is_empty() {
            self.refuse(
                "start: refused, no start candidates in the chapter table",
                logs,
            );
            return;
        }
        let Some(chosen) = found.get(self.config.index) else {
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
            "start: chose candidate {}: chapter record {} phase {:#X} {}",
            chosen.index,
            chosen.record_index,
            chosen.record.phase_id as u16,
            chosen.record.phase_name
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
