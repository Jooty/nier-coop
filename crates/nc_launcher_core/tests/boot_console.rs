use std::time::Duration;

use nc_launcher_core::{
    CONSOLE_LINE_TYPING_MS, CONSOLE_SHORTEST_RUN_MS, ConnectionReport, ConsoleClock,
    ConsoleGameFacts, ConsoleLineKind, ConsoleSave, ConsoleScript, HostingFacts, JoiningFacts,
    NetState, PathKind, RttStats, StepEnd, hosting_script, joining_script,
};
use replicant_proto::RejectReason;

const TICKET: &str = "rcn1fixtureticketlongerthantwelve";
const FOLDER: &str = "~/games/fixture";
const PLAYER: &str = "Jay";
const HOST: &str = "Mira";

fn game(waiting_update: Option<&'static str>) -> ConsoleGameFacts<'static> {
    ConsoleGameFacts {
        protocol: 68,
        installed_build: Some("3f2a91c"),
        waiting_update,
        player_name: PLAYER,
        mode: None,
        seats: None,
    }
}

fn hosting_with(
    answer: Option<Result<&str, &str>>,
    state: &NetState,
    saved_game: Option<Option<ConsoleSave<'static>>>,
    executable: Option<&'static str>,
) -> ConsoleScript {
    hosting_script(HostingFacts {
        game_folder: Ok(FOLDER),
        executable,
        mod_files: Ok(()),
        hook_file: "hook.dll",
        game: game(None),
        saved_game,
        seats: 2,
        enemy_health_percent: 60,
        enemy_damage_percent: 10,
        host_answer: answer,
        state,
    })
}

fn hosting(answer: Option<Result<&str, &str>>, state: &NetState) -> ConsoleScript {
    hosting_with(answer, state, Some(None), Some("game.exe"))
}

fn hosted() -> NetState {
    NetState::Hosting {
        ticket: TICKET.to_owned(),
    }
}

fn report(rejected: Option<RejectReason>) -> ConnectionReport {
    let sample = Duration::from_millis(20);
    ConnectionReport {
        reachable: true,
        path: PathKind::Direct,
        rtt: rejected.is_none().then_some(RttStats {
            min: sample,
            avg: sample,
            max: sample,
        }),
        loss_percent: 0.0,
        rejected,
        host_name: Some(HOST.to_owned()),
        verdict: "verdict".to_owned(),
    }
}

fn joining<'a>(
    probe: Option<Result<&'a ConnectionReport, &'a str>>,
    join_answer: Option<Result<(), &'a str>>,
    state: &'a NetState,
) -> ConsoleScript {
    joining_script(JoiningFacts {
        ticket_text: TICKET,
        ticket: Ok(()),
        game: game(None),
        probe,
        join_answer,
        state,
        own_slot: Some(1),
    })
}

fn ends(script: &ConsoleScript) -> Vec<StepEnd> {
    script.steps.iter().map(|step| step.end).collect()
}

fn labels(script: &ConsoleScript) -> Vec<&'static str> {
    script
        .steps
        .iter()
        .flat_map(|step| {
            std::iter::once(step.label).chain(step.details.iter().map(|detail| detail.label))
        })
        .collect()
}

#[test]
fn a_console_line_closes_only_when_its_step_has_ended() {
    let waiting = hosting(None, &NetState::Idle);
    assert_eq!(
        ends(&waiting),
        [
            StepEnd::Passed,
            StepEnd::Passed,
            StepEnd::Running,
            StepEnd::Running,
            StepEnd::Running
        ]
    );
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&waiting, 0.0);
    let long_after = clock.view(&waiting, 10_000.0, true);
    let last = long_after.lines.last().expect("a line");
    assert_eq!(last.label, "SESSION");
    assert_eq!(last.word, None);
    assert!(last.waiting);
    assert_eq!(long_after.cursor_line, Some(long_after.lines.len() - 1));
    assert!(!long_after.lobby_due);
    assert!(
        !long_after.moving,
        "a line held by real work asks for no frames"
    );

    let done = hosting(Some(Ok(TICKET)), &hosted());
    clock.observe(&done, 10_000.0);
    let at_once = clock.view(&done, 10_000.0, true);
    assert!(at_once.lines.iter().all(|line| line.typed_share >= 1.0));
    assert_eq!(
        at_once.lines.last().map(|line| line.kind),
        Some(ConsoleLineKind::Closing)
    );
    assert!(at_once.lobby_due, "late lines add no time beyond the work");

    let refused_later = report(None);
    let joined = joining(Some(Ok(&refused_later)), Some(Ok(())), &NetState::Joining);
    assert_eq!(
        ends(&joined),
        [
            StepEnd::Passed,
            StepEnd::Passed,
            StepEnd::Passed,
            StepEnd::Running,
            StepEnd::Running
        ]
    );
}

#[test]
fn the_console_keeps_a_floor_when_the_work_is_instant() {
    let done = hosting(Some(Ok(TICKET)), &hosted());
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&done, 0.0);
    let first = clock.view(&done, f64::from(CONSOLE_LINE_TYPING_MS) / 2.0, true);
    assert_eq!(first.lines.len(), 1);
    assert_eq!(first.lines[0].word, None);
    assert!(first.moving);
    let before_floor = clock.view(&done, f64::from(CONSOLE_SHORTEST_RUN_MS) - 1.0, true);
    assert!(
        before_floor
            .lines
            .iter()
            .all(|line| line.typed_share >= 1.0)
    );
    assert!(!before_floor.lobby_due);
    assert!(
        clock
            .view(&done, f64::from(CONSOLE_SHORTEST_RUN_MS), true)
            .lobby_due
    );
}

#[test]
fn a_failed_step_keeps_the_console_open() {
    let failed = hosting(Some(Err("could not open the session")), &NetState::Idle);
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&failed, 0.0);
    let view = clock.view(&failed, 60_000.0, true);
    assert!(view.failed);
    assert!(!view.lobby_due);
    let last = view.lines.last().expect("a line");
    assert_eq!(last.label, "SESSION");
    assert!(last.failed);
    assert_eq!(view.error.as_deref(), Some("could not open the session"));

    let refused = report(Some(RejectReason::VersionMismatch));
    let versions = joining(Some(Ok(&refused)), None, &NetState::Idle);
    assert_eq!(
        ends(&versions),
        [
            StepEnd::Passed,
            StepEnd::Passed,
            StepEnd::Failed,
            StepEnd::Running,
            StepEnd::Running
        ]
    );
    let full = report(Some(RejectReason::SessionFull));
    let seat = joining(Some(Ok(&full)), None, &NetState::Idle);
    assert_eq!(seat.steps[3].end, StepEnd::Failed);
}

#[test]
fn no_motion_shows_the_console_whole() {
    let done = hosting(Some(Ok(TICKET)), &hosted());
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&done, 0.0);
    let view = clock.view(&done, 0.0, false);
    assert!(view.lines.iter().all(|line| line.typed_share >= 1.0));
    assert!(
        view.lobby_due,
        "with motion off the lobby shows once the work is done"
    );
    assert!(!view.moving);

    let waiting = hosting(None, &NetState::Idle);
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&waiting, 0.0);
    let view = clock.view(&waiting, 0.0, false);
    let last = view.lines.last().expect("a line");
    assert_eq!(last.label, "SESSION");
    assert!((last.typed_share - 1.0).abs() < f32::EPSILON);
    assert_eq!(last.word, None);
}

#[test]
fn a_skip_shows_the_end_state_at_once() {
    let done = hosting(Some(Ok(TICKET)), &hosted());
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&done, 0.0);
    clock.skip();
    assert!(clock.view(&done, 1.0, true).lobby_due);
}

#[test]
fn a_fact_the_launcher_does_not_hold_gets_no_line() {
    let new_game = hosting_with(Some(Ok(TICKET)), &hosted(), Some(None), None);
    let shown = labels(&new_game);
    for absent in ["EXECUTABLE", "UPDATE", "SAVE", "LEVEL", "PLAY TIME"] {
        assert!(!shown.contains(&absent), "{absent} has no fact behind it");
    }
    assert!(shown.contains(&"PLAYER"));

    let save = ConsoleSave {
        slot: 2,
        level: 12,
        play_time: "22 h 10",
    };
    let saved = hosting_with(Some(Ok(TICKET)), &hosted(), Some(Some(save)), None);
    let session = &saved.steps[2];
    assert_eq!(session.value.as_deref(), Some("SAVED GAME"));
    let save_line = session
        .details
        .iter()
        .find(|detail| detail.label == "SAVE")
        .expect("the save is told");
    assert_eq!(save_line.value, "SLOT 03");

    let unknown_path = ConnectionReport {
        path: PathKind::Unknown,
        host_name: None,
        ..report(None)
    };
    let script = joining(Some(Ok(&unknown_path)), None, &NetState::Joining);
    assert_eq!(script.steps[1].value, None, "no host name was told");
    assert!(!labels(&script).contains(&"PATH"));

    let waiting = hosting(None, &NetState::Idle);
    assert!(
        waiting.steps[2].details.is_empty(),
        "a step tells what it found only once it has passed"
    );
}

#[test]
fn the_meter_counts_closed_steps() {
    let waiting = hosting(None, &NetState::Idle);
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&waiting, 0.0);
    let view = clock.view(&waiting, 10_000.0, true);
    assert_eq!(view.counted_steps, 4);
    assert_eq!(view.closed_steps, 2);

    let done = hosting(Some(Ok(TICKET)), &hosted());
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&done, 0.0);
    assert_eq!(clock.view(&done, 0.0, true).closed_steps, 0);
    assert_eq!(clock.view(&done, 10_000.0, true).closed_steps, 4);
}

#[test]
fn a_game_with_several_modes_names_its_mode_and_seats_on_the_console() {
    let mut several = game(None);
    several.mode = Some("Free roam");
    several.seats = Some(8);
    let hosting_several = hosting_script(HostingFacts {
        game_folder: Ok(FOLDER),
        executable: None,
        mod_files: Ok(()),
        hook_file: "hook.dll",
        game: several,
        saved_game: Some(None),
        seats: 8,
        enemy_health_percent: 60,
        enemy_damage_percent: 10,
        host_answer: Some(Ok(TICKET)),
        state: &hosted(),
    });
    assert!(labels(&hosting_several).contains(&"MODE"));
    let connected = NetState::Connected {
        peer_name: HOST.to_owned(),
    };
    let good_report = report(None);
    let joined = joining_script(JoiningFacts {
        ticket_text: TICKET,
        ticket: Ok(()),
        game: several,
        probe: Some(Ok(&good_report)),
        join_answer: Some(Ok(())),
        state: &connected,
        own_slot: Some(1),
    });
    let joined_labels = labels(&joined);
    assert!(joined_labels.contains(&"MODE") && joined_labels.contains(&"SEATS"));
    assert!(!labels(&hosting(Some(Ok(TICKET)), &hosted())).contains(&"MODE"));
}
