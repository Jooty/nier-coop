use automata_game::character::Character;
use automata_game::start::{
    ArrivalReadings, ChapterRecord, PlayReadings, SAVED_SCENE_REQUEST_PARAMETERS, StartAction,
    StartCall, StartConfig, StartMachine, StartMode, StartReadings, StartState, SubPhaseReading,
    TitleIdle, TitleReadings, has_arrived, is_in_play, title_idle, title_is_idle,
};

fn config(mode: StartMode, index: usize) -> StartConfig {
    StartConfig {
        mode,
        index,
        slot: 2,
        hero: Character::TwoB,
        partner: Character::NineS,
        route_lead: Character::TwoB,
    }
}

fn idle_title() -> TitleReadings {
    TitleReadings {
        scene_state: 1,
        phase_id: 0xF00,
        pending_phase_id: 0xF00,
        scene_loaded: 1,
        sub_phase: SubPhaseReading::Name("START".to_owned()),
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
        slot_calls_verified: true,
    }
}

fn arrival(phase_id: i32, player_present: bool) -> ArrivalReadings {
    arrival_named(phase_id, "first_start", player_present)
}

fn arrival_named(phase_id: i32, name: &str, player_present: bool) -> ArrivalReadings {
    ArrivalReadings {
        scene_state: 1,
        phase_id,
        phase_name: name.to_owned(),
        scene_loaded: 1,
        player_present,
        player_held_by: None,
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
            scene_state: 3,
            ..idle_title()
        },
        TitleReadings {
            phase_id: 0x100,
            ..idle_title()
        },
        TitleReadings {
            pending_phase_id: -1,
            ..idle_title()
        },
        TitleReadings {
            scene_loaded: 0,
            ..idle_title()
        },
        TitleReadings {
            sub_phase: SubPhaseReading::Name("SIGNIN".to_owned()),
            ..idle_title()
        },
        TitleReadings {
            sub_phase: SubPhaseReading::NotAName,
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
fn a_scene_being_torn_down_is_never_an_idle_title() {
    let teardown = TitleReadings {
        scene_state: 3,
        pending_phase_id: -1,
        scene_loaded: 0,
        ..idle_title()
    };
    assert_eq!(title_idle(&teardown), TitleIdle::No);
}

#[test]
fn a_sub_phase_that_is_not_a_name_is_told_apart_from_another_name() {
    let unreadable = TitleReadings {
        sub_phase: SubPhaseReading::NotAName,
        ..idle_title()
    };
    assert_eq!(title_idle(&unreadable), TitleIdle::YesButSubPhaseIsNotAName);
    let signing_in = TitleReadings {
        sub_phase: SubPhaseReading::Name("SIGNIN".to_owned()),
        ..idle_title()
    };
    assert_eq!(title_idle(&signing_in), TitleIdle::No);
}

#[test]
fn a_sub_phase_name_is_short_printable_text_with_an_end() {
    let mut buffer = [0_u8; 0x20];
    buffer[..5].copy_from_slice(b"START");
    assert_eq!(
        SubPhaseReading::of_bytes(&buffer),
        SubPhaseReading::Name("START".to_owned())
    );
    assert_eq!(
        SubPhaseReading::of_bytes(&[0_u8; 0x20]),
        SubPhaseReading::NotAName
    );
    assert_eq!(
        SubPhaseReading::of_bytes(&[b'A'; 0x20]),
        SubPhaseReading::NotAName
    );
    buffer[2] = 0x07;
    assert_eq!(
        SubPhaseReading::of_bytes(&buffer),
        SubPhaseReading::NotAName
    );
}

#[test]
fn without_a_readable_sub_phase_the_title_must_stay_idle_for_three_seconds() {
    let mut machine = world_machine(0);
    let unreadable = readings(TitleReadings {
        sub_phase: SubPhaseReading::NotAName,
        ..idle_title()
    });
    let first = machine.step(0, &unreadable);
    assert_eq!(first.action, StartAction::Nothing);
    assert!(first.logs[0].contains("cannot be read as a name"));
    assert_eq!(
        machine.step(2_000, &unreadable).action,
        StartAction::Nothing
    );
    let busy = readings(TitleReadings {
        sub_phase: SubPhaseReading::NotAName,
        save_busy: Some(true),
        ..idle_title()
    });
    assert_eq!(machine.step(2_500, &busy).action, StartAction::Nothing);
    assert_eq!(
        machine.step(4_000, &unreadable).action,
        StartAction::Nothing
    );
    assert_eq!(
        machine.step(7_000, &unreadable).action,
        StartAction::ReadTable
    );
}

#[test]
fn arrival_needs_a_running_loaded_scene_of_the_chosen_phase_and_a_body() {
    assert!(has_arrived(&arrival(0x200, true), 0x200));
    assert!(!has_arrived(&arrival(0x200, false), 0x200));
    assert!(!has_arrived(&arrival(0x201, true), 0x200));
    assert!(!has_arrived(
        &ArrivalReadings {
            scene_loaded: 0,
            ..arrival(0x200, true)
        },
        0x200
    ));
    assert!(!has_arrived(
        &ArrivalReadings {
            scene_state: 3,
            ..arrival(0x200, true)
        },
        0x200
    ));
    assert!(has_arrived(
        &arrival_named(0x200, "a_later_name", true),
        0x200
    ));
}

#[test]
fn in_play_is_a_running_loaded_scene_off_the_title_with_a_hero() {
    let playing = PlayReadings {
        scene_state: 1,
        phase_id: 0x200,
        scene_loaded: 1,
        player_present: true,
    };
    assert!(is_in_play(&playing));
    for not_playing in [
        PlayReadings {
            scene_state: 2,
            ..playing
        },
        PlayReadings {
            phase_id: 0xF00,
            ..playing
        },
        PlayReadings {
            scene_loaded: 0,
            ..playing
        },
        PlayReadings {
            player_present: false,
            ..playing
        },
    ] {
        assert!(!is_in_play(&not_playing));
    }
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
    assert!(first.logs[0].starts_with("start: waiting for the title: scene state 1"));
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
fn a_guest_that_takes_the_hosts_route_says_so_and_its_plan_sets_that_route() {
    let guest = StartConfig {
        hero: Character::NineS,
        partner: Character::TwoB,
        route_lead: Character::TwoB,
        ..config(StartMode::World, 1)
    };
    let mut machine = StartMachine::new(guest, true).0;
    machine.step(0, &readings(idle_title()));
    let mut records = table();
    for record in &mut records {
        record.route = 2;
        record.hero_mask = 0b011;
    }
    let logs = machine.table_read(Ok(records));
    assert_eq!(
        logs[1],
        "start: the route follows the host's character 2b (Route/A); 9s on its own would start Route/B"
    );
    let StartAction::RunPlan(plan) = machine.step(5, &readings(idle_title())).action else {
        panic!("expected the plan");
    };
    assert_eq!(
        plan.before_phase_start[1..4],
        [
            StartCall::SetFlag("PL/9S".to_owned()),
            StartCall::SetFlag("Buddy/2B".to_owned()),
            StartCall::SetFlag("Route/A".to_owned()),
        ]
    );
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
            route_lead: Character::A2,
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
    assert!(machine.phase_start_returned(25, false).is_empty());
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
    let logs = machine.phase_start_returned(25, true);
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
    machine.phase_start_returned(25, true);
    let waiting = StartReadings {
        arrival: Some(arrival(0xF00, false)),
        ..readings(idle_title())
    };
    assert_eq!(machine.step(100, &waiting).logs.len(), 1);
    assert!(machine.step(5_000, &waiting).logs.is_empty());
    assert_eq!(machine.step(10_100, &waiting).logs.len(), 1);
}

#[test]
fn an_arrival_that_never_comes_refuses_after_five_minutes() {
    let mut machine = machine_at_phase_start();
    machine.step(20, &readings(idle_title()));
    machine.phase_start_returned(25, true);
    let waiting = StartReadings {
        arrival: Some(arrival(0xF00, false)),
        ..readings(idle_title())
    };
    assert!(!machine.step(300_000, &waiting).logs[0].contains("refused"));
    let step = machine.step(300_030, &waiting);
    assert!(step.logs[0].starts_with("start: refused, phase 0x200 was not reached after 300 s"));
    assert!(machine.is_refused());
    assert!(!machine.is_past_title());
}

fn slot_machine_at_loading() -> StartMachine {
    let mut machine = StartMachine::new(config(StartMode::Slot, 0), true).0;
    assert_eq!(
        machine.step(0, &readings(idle_title())).action,
        StartAction::LoadSlot(2)
    );
    assert!(machine.slot_requested(0, true).is_empty());
    assert!(machine.is_loading_slot());
    machine
}

fn busy_title() -> StartReadings {
    readings(TitleReadings {
        save_busy: Some(true),
        ..idle_title()
    })
}

#[test]
fn the_slot_mode_loads_the_slot_then_enters_its_scene_then_waits_to_arrive() {
    let mut machine = slot_machine_at_loading();
    assert_eq!(
        machine.step(100, &busy_title()).action,
        StartAction::Nothing
    );
    assert_eq!(
        machine.step(200, &readings(idle_title())).action,
        StartAction::EnterSavedScene
    );
    machine.saved_scene_requested(210, 0x500, "saved_scene".to_owned());
    assert!(machine.wants_arrival_readings());
    let arrived = StartReadings {
        arrival: Some(arrival_named(0x500, "saved_scene", true)),
        ..readings(idle_title())
    };
    machine.step(900, &arrived);
    assert!(machine.is_in_world());
    assert_eq!(machine.status_line(), "start: in the world (slot 2)");
}

#[test]
fn the_saved_scene_is_requested_with_the_values_the_title_uses() {
    assert_eq!(SAVED_SCENE_REQUEST_PARAMETERS, [1, 0, 0x78, 1, 0, 0]);
}

#[test]
fn the_slot_file_is_looked_for_only_in_slot_mode_at_the_title() {
    assert!(!world_machine(0).wants_slot_file_check());
    let mut machine = StartMachine::new(config(StartMode::Slot, 0), true).0;
    assert!(machine.wants_slot_file_check());
    machine.step(0, &readings(idle_title()));
    machine.slot_requested(0, true);
    assert!(!machine.wants_slot_file_check());
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
fn a_slot_above_three_refuses_before_anything_is_hooked() {
    let (machine, logs) = StartMachine::new(
        StartConfig {
            slot: 4,
            ..config(StartMode::Slot, 0)
        },
        true,
    );
    assert_eq!(
        logs,
        ["start: refused, start_slot 4 is not a save slot (0 to 3)"]
    );
    assert!(machine.is_finished());
    let (world, _) = StartMachine::new(
        StartConfig {
            slot: 4,
            ..config(StartMode::World, 0)
        },
        true,
    );
    assert!(!world.is_finished());
}

#[test]
fn slot_start_refuses_on_an_exe_whose_save_functions_were_not_verified() {
    let mut machine = StartMachine::new(config(StartMode::Slot, 0), true).0;
    let unverified = StartReadings {
        slot_calls_verified: false,
        ..readings(idle_title())
    };
    let step = machine.step(0, &unverified);
    assert_eq!(step.action, StartAction::Nothing);
    assert!(
        step.logs
            .iter()
            .any(|l| l.starts_with("start: refused, slot start is off on this exe"))
    );
}

#[test]
fn a_load_request_the_game_did_not_take_refuses_at_once() {
    let mut machine = StartMachine::new(config(StartMode::Slot, 0), true).0;
    machine.step(0, &readings(idle_title()));
    let logs = machine.slot_requested(0, false);
    assert!(
        logs[0].starts_with("start: refused, the game did not take the load request for slot 2")
    );
    assert!(machine.is_refused());
    assert_eq!(
        machine.step(5_000, &readings(idle_title())).action,
        StartAction::Nothing
    );
}

#[test]
fn a_failed_slot_load_refuses() {
    let mut machine = slot_machine_at_loading();
    let failed = readings(TitleReadings {
        save_failed: Some(true),
        ..idle_title()
    });
    let step = machine.step(50, &failed);
    assert_eq!(step.logs, ["start: refused, loading slot 2 failed"]);
}

#[test]
fn a_slot_load_whose_failure_cannot_be_read_refuses() {
    let mut machine = slot_machine_at_loading();
    let unread = readings(TitleReadings {
        save_failed: None,
        ..idle_title()
    });
    assert_eq!(
        machine.step(50, &unread).logs,
        ["start: refused, loading slot 2 failed"]
    );
}

#[test]
fn a_slot_that_keeps_loading_for_a_minute_refuses() {
    let mut machine = slot_machine_at_loading();
    assert_eq!(
        machine.step(60_000, &busy_title()).action,
        StartAction::Nothing
    );
    assert!(machine.is_loading_slot());
    let step = machine.step(60_001, &busy_title());
    assert_eq!(
        step.logs,
        ["start: refused, slot 2 was still loading after 60 s"]
    );
}

#[test]
fn every_refusal_leaves_the_machine_refused_and_not_past_the_title() {
    let mut machine = world_machine(0);
    machine.step(0, &readings(idle_title()));
    assert!(machine.is_past_title());
    machine.table_read(Err("bad header".to_owned()));
    assert!(machine.is_refused());
    assert!(!machine.is_past_title());
    assert!(machine.is_finished());
}

#[test]
fn a_failed_game_call_refuses_with_its_reason() {
    let mut machine = world_machine(0);
    let logs = machine.call_failed("the chapter data pointer is null");
    assert_eq!(logs, ["start: refused, the chapter data pointer is null"]);
}

#[test]
fn the_world_is_reached_only_once_the_player_is_free_however_long_a_cutscene_runs() {
    let mut machine = machine_at_phase_start();
    machine.step(20, &readings(idle_title()));
    machine.phase_start_returned(25, true);
    let in_a_cutscene = StartReadings {
        arrival: Some(ArrivalReadings {
            player_held_by: Some("the scene input-off bit is set".to_owned()),
            ..arrival(0x200, true)
        }),
        ..readings(idle_title())
    };
    let step = machine.step(100, &in_a_cutscene);
    assert!(
        step.logs[0].starts_with("start: reached phase 0x200, waiting until the player is free")
    );
    assert!(step.logs[0].contains("the scene input-off bit is set"));
    let long_after = machine.step(900_000, &in_a_cutscene);
    assert!(!machine.is_refused());
    assert!(!machine.is_in_world());
    assert!(long_after.logs.iter().all(|line| !line.contains("refused")));
    let free = StartReadings {
        arrival: Some(arrival(0x200, true)),
        ..readings(idle_title())
    };
    let step = machine.step(900_016, &free);
    assert_eq!(step.logs, ["start: in the world, phase 0x200 first_start"]);
    assert!(machine.is_in_world());
}

#[test]
fn a_game_sent_back_to_the_title_from_the_world_runs_the_same_start_again() {
    let mut machine = machine_at_phase_start();
    assert!(
        machine
            .start_again_from_the_title("a made-up reason")
            .is_empty()
    );
    machine.step(20, &readings(idle_title()));
    machine.phase_start_returned(25, true);
    let free = StartReadings {
        arrival: Some(arrival(0x200, true)),
        ..readings(idle_title())
    };
    machine.step(100, &free);
    assert!(machine.is_in_world());
    let logs = machine.start_again_from_the_title("a made-up reason");
    assert_eq!(logs.len(), 1);
    assert!(logs[0].starts_with("start: a made-up reason; the start is run again from the title"));
    assert!(!machine.is_finished());
    assert!(!machine.is_past_title());
    assert_eq!(
        machine.step(5_000, &readings(idle_title())).action,
        StartAction::ReadTable
    );
    machine.table_read(Ok(table()));
    let step = machine.step(5_010, &readings(idle_title()));
    assert!(matches!(step.action, StartAction::RunPlan(_)));
}
