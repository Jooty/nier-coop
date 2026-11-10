use std::time::Instant;

use automata_game::address::{
    CHAPTER_DATA_POINTER, CURRENT_PHASE_ID_VALUE, CURRENT_SUB_PHASE_NAME_VALUE,
    HERO_MODEL_ID_VALUE, PENDING_PHASE_ID_VALUE, SAVED_PHASE_ID_VALUE, SAVED_PHASE_NAME_VALUE,
    SCENE_LOADED_FLAG_VALUE, SCENE_STATE_VALUE, STARTED_FROM_CHAPTER_SELECT_VALUE,
    UI_BOOKKEEPING_FLAG_VALUE, UI_BOOKKEEPING_WORD_VALUE,
};
use automata_game::build::variant_of_image_size;
use automata_game::partner::{ReadyLatch, not_free_reason};
use automata_game::start::{
    ArrivalReadings, PhaseStartCall, SUB_PHASE_NAME_BUFFER_LEN, StartAction, StartCall,
    StartMachine, StartReadings, SubPhaseReading, TitleIdle, TitleReadings,
    UI_WORD_MASK_WITHOUT_TOP_BIT, title_idle,
};
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::{
    arm_story_gate, coop_save_file_exists, disarm_story_gate, install_story_gate, run_as_ours,
};
use crate::net::set_local_in_play;
use crate::start_driver::chapter_table_read::read_chapter_table;
use crate::start_driver::driver_state::{DRIVER, Driver, STATUS_LINE};
use crate::start_driver::{NOT_READ_SENTINEL, SCENE_NAME_READ_LIMIT};
use crate::start_setting::GameSettings;

pub fn install(build: VerifiedBuild, settings: &GameSettings, saves_redirected: bool) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        log_line("start: image size matches no known exe, nothing started");
        return;
    };
    let (mut machine, logs) = StartMachine::new(settings.start_config(), saves_redirected);
    log_all(logs);
    if !machine.is_finished() && !install_story_gate(build, settings.story) {
        log_all(machine.call_failed("the story gate could not be hooked"));
    }
    let calls = GameCalls::new(build, variant);
    let slot_calls_verified = calls.slot_calls_verified();
    if !slot_calls_verified {
        log_line(
            "start: the bytes at the two inferred save functions of this exe are not the expected ones, slot start is off",
        );
    }
    set_status(&machine);
    let Ok(mut guard) = DRIVER.lock() else {
        return;
    };
    *guard = Some(Driver {
        machine,
        calls,
        started: Instant::now(),
        phase_name_buffer: Vec::new(),
        after_phase_start: Vec::new(),
        slot: settings.slot,
        slot_calls_verified,
        in_play: false,
        ready: ReadyLatch::default(),
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
    // Once the start is over (in the world, refused or not used) only play is watched, so the
    // launcher also hears of a game the player started from the title.
    if driver.machine.is_finished() {
        driver.watch_play();
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
        let readings = self.read();
        let step = self.machine.step(now_ms, &readings);
        log_all(step.logs);
        if self.machine.is_past_title() {
            arm_story_gate();
        }
        self.execute(step.action, now_ms);
        if self.machine.is_refused() {
            disarm_story_gate("the start was refused, the title is left as it is");
        }
        if self.machine.is_in_world() {
            self.watch_play();
        }
    }

    fn watch_play(&mut self) {
        let free = self.calls.read_free();
        // Ready from the first free moment of a stay in play, so a cutscene on arrival is waited out.
        let in_play = self.ready.observe(free.in_play, free.is_free());
        if in_play != self.in_play {
            self.in_play = in_play;
            set_local_in_play(in_play);
            log_line(if in_play {
                "start: in play, the player is free"
            } else {
                "start: not in play (a scene change, the title or no hero)"
            });
        }
    }

    fn read(&self) -> StartReadings {
        let calls = &self.calls;
        let value = |address| calls.read_i32(address).unwrap_or(NOT_READ_SENTINEL);
        let scene_state = value(SCENE_STATE_VALUE);
        let phase_id = value(CURRENT_PHASE_ID_VALUE);
        let scene_loaded = value(SCENE_LOADED_FLAG_VALUE);
        let mut title = TitleReadings {
            scene_state,
            phase_id,
            pending_phase_id: value(PENDING_PHASE_ID_VALUE),
            scene_loaded,
            sub_phase: calls
                .read_bytes(CURRENT_SUB_PHASE_NAME_VALUE, SUB_PHASE_NAME_BUFFER_LEN)
                .map_or(SubPhaseReading::NotAName, |bytes| {
                    SubPhaseReading::of_bytes(&bytes)
                }),
            save_busy: Some(false),
            save_failed: None,
            chapter_data_loaded: calls.read_u64(CHAPTER_DATA_POINTER).is_ok_and(|p| p != 0),
        };
        // The save manager is asked only when everything else already says idle.
        let looks_idle = title_idle(&title) != TitleIdle::No;
        let loading_slot = self.machine.is_loading_slot();
        title.save_busy = (looks_idle || loading_slot).then(|| calls.save_is_busy());
        if loading_slot {
            title.save_failed = calls.save_failed();
        }
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
                scene_loaded,
                player_present: calls.current_hero_present(),
                player_held_by: not_free_reason(&calls.read_free())
                    .map(|reason| reason.to_string()),
            });
        StartReadings {
            title,
            arrival,
            slot_file_present: self.machine.wants_slot_file_check()
                && looks_idle
                && coop_save_file_exists(&format!("SlotData_{}.dat", self.slot)),
            slot_calls_verified: self.slot_calls_verified,
        }
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
            StartAction::TryPhaseStart(call) => self.try_phase_start(&call, now_ms),
            StartAction::LoadSlot(slot) => {
                run_as_ours(|| self.calls.request_load_slot(slot));
                // The request returns nothing; a request that was taken shows as busy at once.
                let busy_right_after = self.calls.save_is_busy();
                log_all(self.machine.slot_requested(now_ms, busy_right_after));
            }
            StartAction::EnterSavedScene => self.enter_saved_scene(now_ms),
        }
    }

    fn try_phase_start(&mut self, call: &PhaseStartCall, now_ms: u64) {
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
        log_all(self.machine.phase_start_returned(now_ms, accepted));
    }

    fn enter_saved_scene(&mut self, now_ms: u64) {
        let calls = &self.calls;
        let Ok(phase_id) = calls.read_i32(SAVED_PHASE_ID_VALUE) else {
            log_all(self.machine.call_failed("cannot read the saved phase id"));
            return;
        };
        let name_address = calls.address_value(SAVED_PHASE_NAME_VALUE);
        let name = calls.read_c_string_at(name_address, SCENE_NAME_READ_LIMIT);
        // The title does the same before its scene request: the save manager then applies the
        // loaded story when the old scene is gone.
        if !run_as_ours(|| calls.apply_loaded_story()) {
            log_all(self.machine.call_failed(
                "the save manager did not take the request to apply the loaded story",
            ));
            return;
        }
        log_line(&format!(
            "start: entering the saved scene, phase {phase_id:#X} {name}"
        ));
        run_as_ours(|| calls.request_saved_scene(phase_id, name_address as usize));
        self.machine.saved_scene_requested(now_ms, phase_id, name);
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
                .write_u8(UI_BOOKKEEPING_FLAG_VALUE, 1)
                .map_err(write_failed),
        }
    }
}
