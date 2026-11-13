use std::time::{Duration, SystemTime};

use automata_game::character::Character;
use automata_game::start::{
    ArrivalReadings, NEW_GAME_OPENING_PHASE_NAME, NEW_GAME_SCENE_REQUEST_PARAMETERS,
    SAVED_SCENE_REQUEST_PARAMETERS, StartAction, StartConfig, StartMachine, StartMode,
    StartReadings, SubPhaseReading, TitleReadings, newest_slot, saved_scene_request,
};

fn config(mode: StartMode, slot: u32) -> StartConfig {
    StartConfig {
        mode,
        index: 0,
        slot,
        hero: Character::TwoB,
        partner: Character::NineS,
        route_lead: Character::TwoB,
    }
}

fn title(save_busy: bool, save_failed: bool) -> TitleReadings {
    TitleReadings {
        scene_state: 1,
        phase_id: 0xF00,
        pending_phase_id: 0xF00,
        scene_loaded: 1,
        sub_phase: SubPhaseReading::Name("START".to_owned()),
        save_busy: Some(save_busy),
        save_failed: Some(save_failed),
        chapter_data_loaded: true,
    }
}

fn readings(save_busy: bool, newest_slot: Option<u32>) -> StartReadings {
    StartReadings {
        title: title(save_busy, false),
        arrival: None,
        slot_file_present: false,
        slot_calls_verified: true,
        newest_slot,
    }
}

fn arrived(free: bool) -> StartReadings {
    StartReadings {
        arrival: Some(ArrivalReadings {
            scene_state: 1,
            phase_id: 0x100,
            phase_name: NEW_GAME_OPENING_PHASE_NAME.to_owned(),
            scene_loaded: 1,
            player_present: true,
            player_held_by: (!free).then(|| "a cutscene".to_owned()),
        }),
        ..readings(false, None)
    }
}

fn machine_in_the_opening() -> StartMachine {
    let mut machine = StartMachine::new(config(StartMode::NewGame, 1), true).0;
    machine.step(0, &readings(false, None));
    machine.new_game_save_requested(10, true);
    machine.step(20, &readings(false, None));
    machine.opening_requested(30);
    machine
}

#[test]
fn a_new_game_begins_at_the_idle_title_with_the_slot_it_was_given() {
    let mut machine = StartMachine::new(config(StartMode::NewGame, 1), true).0;
    let step = machine.step(0, &readings(false, None));
    assert_eq!(step.action, StartAction::BeginNewGame(1));
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("difficulty is normal"))
    );
}

#[test]
fn the_opening_is_asked_for_only_after_the_system_data_write_ends() {
    let mut machine = StartMachine::new(config(StartMode::NewGame, 0), true).0;
    machine.step(0, &readings(false, None));
    assert!(machine.new_game_save_requested(10, true).is_empty());
    assert!(machine.waits_for_the_save_manager());
    assert_eq!(
        machine.step(20, &readings(true, None)).action,
        StartAction::Nothing
    );
    assert_eq!(
        machine.step(40, &readings(false, None)).action,
        StartAction::RequestOpening
    );
    machine.opening_requested(50);
    assert!(machine.wants_arrival_readings());
}

#[test]
fn a_system_data_write_the_game_did_not_take_refuses() {
    let mut machine = StartMachine::new(config(StartMode::NewGame, 0), true).0;
    machine.step(0, &readings(false, None));
    let logs = machine.new_game_save_requested(10, false);
    assert!(machine.is_refused());
    assert!(logs[0].contains("system data write"));
}

#[test]
fn the_first_save_is_written_once_the_player_is_free_in_the_opening() {
    let mut machine = machine_in_the_opening();
    assert_eq!(
        machine.step(100, &arrived(false)).action,
        StartAction::Nothing
    );
    assert_eq!(
        machine.step(200, &arrived(true)).action,
        StartAction::WriteFirstSave(1)
    );
    assert!(!machine.is_in_world());
    assert!(machine.first_save_requested(210, true).is_empty());
    let busy = StartReadings {
        title: title(true, false),
        ..readings(true, None)
    };
    machine.step(220, &busy);
    assert!(!machine.is_in_world());
    let step = machine.step(300, &readings(false, None));
    assert_eq!(
        step.logs,
        ["start: the first save of the new game is written: SlotData_1.dat in the co-op folder"]
    );
    assert!(machine.is_in_world());
}

#[test]
fn a_first_save_the_game_did_not_take_never_holds_play_back() {
    let mut machine = machine_in_the_opening();
    machine.step(200, &arrived(true));
    let logs = machine.first_save_requested(210, false);
    assert!(machine.is_in_world());
    assert!(logs[0].contains("play goes on without it"));
}

#[test]
fn a_new_game_sent_back_to_the_title_continues_instead_of_starting_over() {
    let mut machine = machine_in_the_opening();
    machine.step(200, &arrived(true));
    machine.first_save_requested(210, true);
    machine.step(300, &readings(false, None));
    assert!(machine.is_in_world());
    machine.start_again_from_the_title("a made-up reason");
    assert!(machine.wants_newest_slot());
    assert_eq!(
        machine.step(5_000, &readings(false, Some(1))).action,
        StartAction::LoadSlot(1)
    );
}

#[test]
fn continue_loads_the_newest_slot() {
    let mut machine = StartMachine::new(config(StartMode::Continue, 0), true).0;
    assert!(machine.wants_newest_slot());
    let step = machine.step(0, &readings(false, Some(2)));
    assert_eq!(step.action, StartAction::LoadSlot(2));
    assert_eq!(machine.slot(), 2);
}

#[test]
fn continue_without_a_save_refuses() {
    let mut machine = StartMachine::new(config(StartMode::Continue, 0), true).0;
    machine.step(0, &readings(false, None));
    assert!(machine.is_refused());
}

#[test]
fn a_new_game_slot_above_three_refuses_before_anything_is_hooked() {
    let (machine, logs) = StartMachine::new(config(StartMode::NewGame, 4), true);
    assert!(machine.is_refused());
    assert!(logs[0].contains("start_slot 4"));
}

#[test]
fn the_newest_slot_is_the_one_written_last() {
    let at = |seconds| SystemTime::UNIX_EPOCH + Duration::from_secs(seconds);
    assert_eq!(newest_slot(&[]), None);
    assert_eq!(newest_slot(&[(0, at(5)), (2, at(9)), (1, at(7))]), Some(2));
    assert_eq!(newest_slot(&[(3, at(5)), (1, at(5))]), Some(1));
    assert_eq!(newest_slot(&[(4, at(9)), (0, at(1))]), Some(0));
}

#[test]
fn a_save_of_the_opening_is_entered_as_a_new_game_enters_it() {
    let opening = saved_scene_request(0x100, NEW_GAME_OPENING_PHASE_NAME);
    assert!(opening.is_the_opening);
    assert_eq!(opening.parameters, NEW_GAME_SCENE_REQUEST_PARAMETERS);
    assert_eq!(NEW_GAME_SCENE_REQUEST_PARAMETERS, [2, 1, 0x78, 1, 0, 0]);
    let elsewhere = saved_scene_request(0x100, "another_phase");
    assert!(!elsewhere.is_the_opening);
    assert_eq!(elsewhere.parameters, SAVED_SCENE_REQUEST_PARAMETERS);
}

#[test]
fn the_new_modes_are_read_from_their_link_text() {
    assert_eq!(StartMode::from_text("new_game"), Some(StartMode::NewGame));
    assert_eq!(StartMode::from_text("continue"), Some(StartMode::Continue));
    assert_eq!(StartMode::NewGame.label(), "new_game");
    assert_eq!(StartMode::Continue.label(), "continue");
}
