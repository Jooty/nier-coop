use automata_game::character::Character;
use automata_game::start::{
    ArrivalReadings, ChapterRecord, StartAction, StartConfig, StartMachine, StartMode,
    StartReadings, StartState, TitleReadings, has_arrived, title_is_idle,
};

fn config(mode: StartMode, index: usize) -> StartConfig {
    StartConfig {
        mode,
        index,
        slot: 2,
        hero: Character::TwoB,
        partner: Character::NineS,
    }
}

fn idle_title() -> TitleReadings {
    TitleReadings {
        scene_state: 3,
        phase_id: 0xF00,
        pending_phase_id: -1,
        loading: 0,
        save_busy: Some(false),
        save_failed: Some(false),
        chapter_data_loaded: true,
    }
}

fn readings(title: TitleReadings) -> StartReadings {
    StartReadings {
        title,
        arrival: None,
        slot_file_present: true,
    }
}

fn arrival(phase_id: i32, player_present: bool) -> ArrivalReadings {
    arrival_named(phase_id, "first_start", player_present)
}

fn arrival_named(phase_id: i32, name: &str, player_present: bool) -> ArrivalReadings {
    ArrivalReadings {
        scene_state: 3,
        phase_id,
        phase_name: name.to_owned(),
        loading: 0,
        player_present,
    }
}

fn record(chapter: u8, group: u8, phase_id: i16, name: &str) -> ChapterRecord {
    ChapterRecord {
        chapter,
        group,
        route: 0,
        hero_mask: 1,
        phase_id,
        phase_name: name.to_owned(),
    }
}

fn table() -> Vec<ChapterRecord> {
    vec![
        record(0, 1, 0x100, "opening_Prologue"),
        record(1, 2, 0x200, "first_start"),
        record(2, 3, 0x300, "second_start"),
    ]
}

fn world_machine(index: usize) -> StartMachine {
    StartMachine::new(config(StartMode::World, index), true).0
}

fn machine_at_phase_start() -> StartMachine {
    let mut machine = world_machine(0);
    machine.step(0, &readings(idle_title()));
    machine.table_read(Ok(table()));
    let step = machine.step(10, &readings(idle_title()));
    assert!(matches!(step.action, StartAction::RunPlan(_)));
    machine
}

#[test]
fn the_title_is_idle_only_when_every_reading_agrees() {
    assert!(title_is_idle(&idle_title()));
    for broken in [
        TitleReadings {
            scene_state: 1,
            ..idle_title()
        },
        TitleReadings {
            phase_id: 0x100,
            ..idle_title()
        },
        TitleReadings {
            pending_phase_id: 5,
            ..idle_title()
        },
        TitleReadings {
            loading: 1,
            ..idle_title()
        },
        TitleReadings {
            save_busy: Some(true),
            ..idle_title()
        },
        TitleReadings {
            chapter_data_loaded: false,
            ..idle_title()
        },
    ] {
        assert!(!title_is_idle(&broken));
    }
}

#[test]
fn a_save_state_that_was_not_read_yet_is_not_idle() {
    let unread = TitleReadings {
        save_busy: None,
        ..idle_title()
    };
    assert!(!title_is_idle(&unread));
    assert!(unread.describe().contains("save busy not read yet"));
}

#[test]
fn a_phase_start_that_is_retried_logs_every_ten_seconds() {
    let mut machine = machine_at_phase_start();
    assert_eq!(machine.step(20, &readings(idle_title())).logs.len(), 1);
    assert!(machine.step(30, &readings(idle_title())).logs.is_empty());
    assert_eq!(machine.step(10_100, &readings(idle_title())).logs.len(), 1);
}

#[test]
fn arrival_needs_the_chosen_phase_and_a_body() {
    assert!(has_arrived(&arrival(0x200, true), 0x200, "first_start"));
    assert!(!has_arrived(&arrival(0x200, false), 0x200, "first_start"));
    assert!(!has_arrived(&arrival(0x201, true), 0x200, "first_start"));
    assert!(!has_arrived(
        &ArrivalReadings {
            loading: 1,
            ..arrival(0x200, true)
        },
        0x200,
        "first_start"
    ));
    assert!(!has_arrived(
        &arrival_named(0x200, "other_name", true),
        0x200,
        "first_start"
    ));
}

#[test]
fn unredirected_saves_refuse_the_start_and_stay_at_the_title() {
    let (mut machine, logs) = StartMachine::new(config(StartMode::World, 0), false);
    assert_eq!(logs, ["start: refused, saves are not redirected"]);
    assert_eq!(
        machine.step(0, &readings(idle_title())).action,
        StartAction::Nothing
    );
    assert_eq!(
        machine.status_line(),
        "start: refused, saves are not redirected"
    );
    assert!(!machine.is_past_title());
}

#[test]
fn the_title_choice_does_nothing() {
    let (mut machine, _) = StartMachine::new(config(StartMode::Title, 0), true);
    assert_eq!(
        machine.step(0, &readings(idle_title())).action,
        StartAction::Nothing
    );
    assert_eq!(*machine.state(), StartState::Disabled);
}

#[test]
fn a_busy_title_is_waited_for_and_logged_every_ten_seconds() {
    let mut machine = world_machine(0);
    let busy = readings(TitleReadings {
        save_busy: Some(true),
        ..idle_title()
    });
    let first = machine.step(0, &busy);
    assert_eq!(first.action, StartAction::Nothing);
    assert_eq!(first.logs.len(), 1);
    assert!(first.logs[0].starts_with("start: waiting for the title: scene state 3"));
    assert!(machine.step(9_000, &busy).logs.is_empty());
    assert_eq!(machine.step(10_000, &busy).logs.len(), 1);
    assert_eq!(machine.status_line(), "start: waiting for the title");
}

#[test]
fn an_idle_title_asks_for_the_chapter_table() {
    let mut machine = world_machine(0);
    let step = machine.step(0, &readings(idle_title()));
    assert_eq!(step.action, StartAction::ReadTable);
    assert!(machine.is_past_title());
}

#[test]
fn the_table_is_logged_in_one_line_and_the_chosen_candidate_builds_the_plan() {
    let mut machine = world_machine(1);
    machine.step(0, &readings(idle_title()));
    let logs = machine.table_read(Ok(table()));
    assert_eq!(
        logs,
        [
            "start: 3 chapter records, 2 candidates (the candidate rule is a guess), chose index 1, phase 0x300"
        ]
    );
    let step = machine.step(5, &readings(idle_title()));
    let StartAction::RunPlan(plan) = step.action else {
        panic!("expected the plan");
    };
    assert_eq!(plan.phase_start.phase_id, 0x300);
    assert_eq!(plan.phase_start.chapter_filter, 3);
}

#[test]
fn an_index_out_of_range_refuses_with_its_own_line() {
    let mut machine = world_machine(5);
    machine.step(0, &readings(idle_title()));
    let logs = machine.table_read(Ok(table()));
    assert!(
        logs.iter()
            .any(|l| l == "start: refused, start_index 5 is out of range (2 candidates)")
    );
    assert_eq!(
        machine.step(1, &readings(idle_title())).action,
        StartAction::Nothing
    );
}

#[test]
fn a_table_without_candidates_refuses() {
    let mut machine = world_machine(0);
    machine.step(0, &readings(idle_title()));
    let logs = machine.table_read(Ok(vec![record(0, 1, 0x100, "opening_Prologue")]));
    assert!(
        logs.iter()
            .any(|l| l == "start: refused, no start candidates in the chapter table")
    );
}

#[test]
fn an_unreadable_table_refuses() {
    let mut machine = world_machine(0);
    machine.step(0, &readings(idle_title()));
    let logs = machine.table_read(Err("bad header".to_owned()));
    assert_eq!(
        logs,
        ["start: refused, cannot read the chapter table (bad header)"]
    );
}

#[test]
fn a_hero_mask_without_the_character_bit_is_noted_not_refused() {
    let mut machine = StartMachine::new(
        StartConfig {
            hero: Character::A2,
            ..config(StartMode::World, 0)
        },
        true,
    )
    .0;
    machine.step(0, &readings(idle_title()));
    let logs = machine.table_read(Ok(table()));
    assert!(
        logs.iter()
            .any(|l| l.starts_with("start: note, hero mask 0x1 has no bit for a2"))
    );
    assert!(matches!(machine.state(), StartState::PlanReady(_)));
}

#[test]
fn a_phase_start_returning_zero_is_retried() {
    let mut machine = machine_at_phase_start();
    let step = machine.step(20, &readings(idle_title()));
    assert!(matches!(step.action, StartAction::TryPhaseStart(_)));
    assert!(machine.phase_start_returned(false).is_empty());
    assert!(matches!(
        machine.step(30, &readings(idle_title())).action,
        StartAction::TryPhaseStart(_)
    ));
}

#[test]
fn a_phase_start_that_keeps_failing_for_twenty_seconds_refuses() {
    let mut machine = machine_at_phase_start();
    let step = machine.step(20_011, &readings(idle_title()));
    assert_eq!(step.action, StartAction::Nothing);
    assert!(step.logs[0].starts_with("start: refused, the phase start kept returning 0 for 20 s"));
}

#[test]
fn an_accepted_phase_start_waits_for_arrival_and_then_says_so() {
    let mut machine = machine_at_phase_start();
    machine.step(20, &readings(idle_title()));
    let logs = machine.phase_start_returned(true);
    assert!(logs[0].contains("phase start accepted for phase 0x200 first_start"));
    assert!(machine.wants_arrival_readings());
    assert_eq!(
        machine.status_line(),
        "start: waiting to arrive (start point 0)"
    );
    let waiting = StartReadings {
        arrival: Some(arrival(0xF00, false)),
        ..readings(idle_title())
    };
    let step = machine.step(100, &waiting);
    assert!(step.logs[0].starts_with("start: waiting to arrive at phase 0x200"));
    let arrived = StartReadings {
        arrival: Some(arrival(0x200, true)),
        ..readings(idle_title())
    };
    let step = machine.step(200, &arrived);
    assert_eq!(step.logs, ["start: in the world, phase 0x200 first_start"]);
    assert_eq!(machine.status_line(), "start: in the world (start point 0)");
}

#[test]
fn a_stuck_arrival_is_logged_every_ten_seconds() {
    let mut machine = machine_at_phase_start();
    machine.step(20, &readings(idle_title()));
    machine.phase_start_returned(true);
    let waiting = StartReadings {
        arrival: Some(arrival(0xF00, false)),
        ..readings(idle_title())
    };
    assert_eq!(machine.step(100, &waiting).logs.len(), 1);
    assert!(machine.step(5_000, &waiting).logs.is_empty());
    assert_eq!(machine.step(10_100, &waiting).logs.len(), 1);
}

#[test]
fn the_slot_mode_loads_the_slot_and_enters_its_scene() {
    let mut machine = StartMachine::new(config(StartMode::Slot, 0), true).0;
    assert_eq!(
        machine.step(0, &readings(idle_title())).action,
        StartAction::LoadSlot(2)
    );
    machine.slot_requested(0);
    let busy = readings(TitleReadings {
        save_busy: Some(true),
        ..idle_title()
    });
    assert_eq!(machine.step(100, &busy).action, StartAction::Nothing);
    assert_eq!(
        machine.step(200, &readings(idle_title())).action,
        StartAction::EnterSavedScene
    );
    machine.saved_scene_requested(0x500, "saved_scene".to_owned());
    assert!(machine.wants_arrival_readings());
}

#[test]
fn the_slot_mode_without_a_save_file_refuses() {
    let mut machine = StartMachine::new(config(StartMode::Slot, 0), true).0;
    let none = StartReadings {
        slot_file_present: false,
        ..readings(idle_title())
    };
    let step = machine.step(0, &none);
    assert!(
        step.logs
            .iter()
            .any(|l| l == "start: refused, no save for slot 2 in the co-op folder")
    );
}

#[test]
fn a_failed_slot_load_refuses() {
    let mut machine = StartMachine::new(config(StartMode::Slot, 0), true).0;
    machine.step(0, &readings(idle_title()));
    machine.slot_requested(0);
    let failed = readings(TitleReadings {
        save_failed: Some(true),
        ..idle_title()
    });
    let step = machine.step(50, &failed);
    assert_eq!(step.logs, ["start: refused, loading slot 2 failed"]);
}

#[test]
fn a_slot_load_that_never_shows_busy_is_entered_after_the_settle_time() {
    let mut machine = StartMachine::new(config(StartMode::Slot, 0), true).0;
    machine.step(0, &readings(idle_title()));
    machine.slot_requested(0);
    assert_eq!(
        machine.step(1_000, &readings(idle_title())).action,
        StartAction::Nothing
    );
    assert_eq!(
        machine.step(3_500, &readings(idle_title())).action,
        StartAction::EnterSavedScene
    );
}

#[test]
fn a_failed_game_call_refuses_with_its_reason() {
    let mut machine = world_machine(0);
    let logs = machine.call_failed("the chapter data pointer is null");
    assert_eq!(logs, ["start: refused, the chapter data pointer is null"]);
}
