use std::time::Instant;

use automata_game::address::{
    CHAPTER_DATA_POINTER, CURRENT_PHASE_ID_VALUE, HERO_MODEL_ID_VALUE, LOADING_FLAG_VALUE,
    PENDING_PHASE_ID_VALUE, SAVED_PHASE_ID_VALUE, SAVED_PHASE_NAME_VALUE, SCENE_STATE_VALUE,
    STARTED_FROM_CHAPTER_SELECT_VALUE, UI_BOOKKEEPING_FLAG_VALUE, UI_BOOKKEEPING_WORD_VALUE,
};
use automata_game::build::variant_of_image_size;
use automata_game::start::{
    ArrivalReadings, NO_PENDING_PHASE_ID, PhaseStartCall, StartAction, StartCall, StartMachine,
    StartMode, StartReadings, TITLE_PHASE_ID, TITLE_SCENE_STATE, TitleReadings,
    UI_WORD_MASK_WITHOUT_TOP_BIT,
};
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::{arm_story_gate, coop_save_folder, install_story_gate, run_as_ours};
use crate::start_driver::chapter_table_read::read_chapter_table;
use crate::start_driver::driver_state::{DRIVER, Driver, STATUS_LINE};
use crate::start_driver::{NOT_READ_SENTINEL, SCENE_NAME_READ_LIMIT};
use crate::start_setting::GameSettings;

pub fn install(build: VerifiedBuild, settings: &GameSettings, saves_redirected: bool) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("start: image size matches no known exe, nothing started");
        return;
    };
    let (machine, logs) = StartMachine::new(settings.start_config(), saves_redirected);
    for line in logs {
        log_line(&line);
    }
    if settings.mode != StartMode::Title && saves_redirected {
        install_story_gate(build);
    }
    set_status(&machine);
    let Ok(mut guard) = DRIVER.lock() else {
        return;
    };
    *guard = Some(Driver {
        machine,
        calls: GameCalls::new(build, variant),
        started: Instant::now(),
        phase_name_buffer: Vec::new(),
        after_phase_start: Vec::new(),
        slot: settings.slot,
    });
}

pub fn status_line() -> String {
    STATUS_LINE
        .lock()
        .map(|line| line.clone())
        .unwrap_or_default()
}

pub fn tick() {
    let Ok(mut guard) = DRIVER.try_lock() else {
        return;
    };
    let Some(driver) = guard.as_mut() else {
        return;
    };
    if driver.machine.is_finished() {
        return;
    }
    driver.tick();
    set_status(&driver.machine);
}

fn set_status(machine: &StartMachine) {
    if let Ok(mut line) = STATUS_LINE.lock() {
        *line = machine.status_line();
    }
}

fn log_all(lines: Vec<String>) {
    for line in lines {
        log_line(&line);
    }
}

impl Driver {
    fn tick(&mut self) {
        let now_ms = self.started.elapsed().as_millis() as u64;
        let readings = self.read(now_ms);
        let step = self.machine.step(now_ms, &readings);
        log_all(step.logs);
        if self.machine.is_past_title() {
            arm_story_gate();
        }
        self.execute(step.action, now_ms);
    }

    fn read(&self, _now_ms: u64) -> StartReadings {
        let calls = &self.calls;
        let value = |address| calls.read_i32(address).unwrap_or(NOT_READ_SENTINEL);
        let scene_state = value(SCENE_STATE_VALUE);
        let phase_id = value(CURRENT_PHASE_ID_VALUE);
        let pending_phase_id = value(PENDING_PHASE_ID_VALUE);
        let loading = value(LOADING_FLAG_VALUE);
        let chapter_data_loaded = calls.read_u64(CHAPTER_DATA_POINTER).is_ok_and(|p| p != 0);
        let looks_idle = scene_state == TITLE_SCENE_STATE
            && phase_id == TITLE_PHASE_ID
            && pending_phase_id == NO_PENDING_PHASE_ID
            && loading == 0
            && chapter_data_loaded;
        let loading_slot = self.machine.is_loading_slot();
        let query_saves = looks_idle || loading_slot;
        let title = TitleReadings {
            scene_state,
            phase_id,
            pending_phase_id,
            loading,
            save_busy: query_saves.then(|| calls.save_is_busy()),
            save_failed: if loading_slot {
                calls.save_failed()
            } else {
                None
            },
            chapter_data_loaded,
        };
        let arrival = self
            .machine
            .wants_arrival_readings()
            .then(|| ArrivalReadings {
                scene_state,
                phase_id,
                phase_name: calls.read_c_string_at(
                    calls.address_value(SAVED_PHASE_NAME_VALUE),
                    SCENE_NAME_READ_LIMIT,
                ),
                loading,
                player_present: calls.current_hero_present(),
            });
        StartReadings {
            title,
            arrival,
            slot_file_present: self.slot_file_present(),
        }
    }

    fn slot_file_present(&self) -> bool {
        let Some(folder) = coop_save_folder() else {
            return false;
        };
        std::path::Path::new(&folder)
            .join(format!("SlotData_{}.dat", self.slot))
            .exists()
    }

    fn execute(&mut self, action: StartAction, now_ms: u64) {
        match action {
            StartAction::Nothing => {}
            StartAction::ReadTable => {
                let table = read_chapter_table(&self.calls);
                log_all(self.machine.table_read(table));
            }
            StartAction::RunPlan(plan) => {
                self.after_phase_start = plan.after_phase_start.clone();
                for call in &plan.before_phase_start {
                    if let Err(reason) = self.run_call(call) {
                        log_all(self.machine.call_failed(&reason));
                        return;
                    }
                }
            }
            StartAction::TryPhaseStart(call) => self.try_phase_start(&call),
            StartAction::LoadSlot(slot) => {
                run_as_ours(|| self.calls.request_load_slot(slot));
                self.machine.slot_requested(now_ms);
            }
            StartAction::EnterSavedScene => self.enter_saved_scene(),
        }
    }

    fn try_phase_start(&mut self, call: &PhaseStartCall) {
        self.phase_name_buffer = call.phase_name.as_bytes().to_vec();
        self.phase_name_buffer.push(0);
        let accepted = run_as_ours(|| {
            self.calls
                .phase_start(call.phase_id, &self.phase_name_buffer, call.chapter_filter)
        });
        if accepted {
            let after = std::mem::take(&mut self.after_phase_start);
            for call in &after {
                if let Err(reason) = self.run_call(call) {
                    log_line(&format!("start: {reason}"));
                }
            }
        }
        log_all(self.machine.phase_start_returned(accepted));
    }

    fn enter_saved_scene(&mut self) {
        let calls = &self.calls;
        let Ok(phase_id) = calls.read_i32(SAVED_PHASE_ID_VALUE) else {
            log_all(self.machine.call_failed("cannot read the saved phase id"));
            return;
        };
        let name_address = calls.address_value(SAVED_PHASE_NAME_VALUE);
        let name = calls.read_c_string_at(name_address, SCENE_NAME_READ_LIMIT);
        log_line(&format!(
            "start: entering the saved scene, phase {phase_id:#X} {name}"
        ));
        run_as_ours(|| calls.request_saved_scene(phase_id, name_address as usize));
        self.machine.saved_scene_requested(phase_id, name);
    }

    fn run_call(&self, call: &StartCall) -> Result<(), String> {
        log_line(&format!("start: call: {}", call.describe()));
        let calls = &self.calls;
        let write_failed = |error| format!("a memory write failed ({error})");
        match call {
            StartCall::WriteHeroModelId(id) => calls
                .write_u32(HERO_MODEL_ID_VALUE, *id)
                .map_err(write_failed),
            StartCall::SetFlag(name) => {
                run_as_ours(|| calls.set_scene_flag(name));
                Ok(())
            }
            StartCall::RunMarkerCall => {
                run_as_ours(|| calls.run_marker_call());
                Ok(())
            }
            StartCall::ClearFlagBit(bit) => {
                run_as_ours(|| calls.clear_flag_bit(*bit));
                Ok(())
            }
            StartCall::WriteStartedFromChapterSelect => calls
                .write_i32(STARTED_FROM_CHAPTER_SELECT_VALUE, 1)
                .map_err(write_failed),
            StartCall::ClearTopBitOfUiBookkeepingWord => {
                let word = calls
                    .read_i32(UI_BOOKKEEPING_WORD_VALUE)
                    .map_err(|error| format!("a memory read failed ({error})"))?;
                calls
                    .write_i32(
                        UI_BOOKKEEPING_WORD_VALUE,
                        word & UI_WORD_MASK_WITHOUT_TOP_BIT as i32,
                    )
                    .map_err(write_failed)
            }
            StartCall::WriteUiBookkeepingFlag => calls
                .write_i32(UI_BOOKKEEPING_FLAG_VALUE, 1)
                .map_err(write_failed),
        }
    }
}
