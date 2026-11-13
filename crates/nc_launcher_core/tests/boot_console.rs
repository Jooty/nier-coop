use std::time::Duration;

use nc_launcher_core::{
    CONSOLE_LINE_TYPING_MS, CONSOLE_SHORTEST_RUN_MS, ConnectionReport, ConsoleClock,
    ConsoleScript, HostingFacts, JoiningFacts, NetState, PathKind, RttStats, StepEnd,
    hosting_script, joining_script,
};
use replicant_proto::RejectReason;

const TICKET: &str = "rcn1fixture";

fn hosting(answer: Option<Result<&str, &str>>, state: &NetState) -> ConsoleScript {
    hosting_script(HostingFacts {
        game_folder: Ok(()),
        mod_files: Ok(()),
        host_answer: answer,
        state,
    })
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
        verdict: "verdict".to_owned(),
    }
}

fn ends(script: &ConsoleScript) -> Vec<StepEnd> {
    script.steps.iter().map(|step| step.end).collect()
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
    assert_eq!(long_after.lines.len(), 3);
    assert_eq!(long_after.lines[2].word, None);
    assert_eq!(long_after.cursor_line, Some(2));
    assert!(!long_after.lobby_due);
    assert!(!long_after.moving, "a line held by real work asks for no frames");

    let hosted = NetState::Hosting {
        ticket: TICKET.to_owned(),
    };
    let done = hosting(Some(Ok(TICKET)), &hosted);
    clock.observe(&done, 10_000.0);
    let at_once = clock.view(&done, 10_000.0, true);
    assert_eq!(at_once.lines.len(), 5);
    assert!(at_once.lines.iter().all(|line| line.word.is_some()));
    assert!(at_once.lobby_due, "late lines add no time beyond the work");

    let joining = joining_script(JoiningFacts {
        ticket: Ok(()),
        probe: Some(Ok(&report(None))),
        join_answer: Some(Ok(())),
        state: &NetState::Joining,
    });
    assert_eq!(
        ends(&joining),
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
    let hosted = NetState::Hosting {
        ticket: TICKET.to_owned(),
    };
    let done = hosting(Some(Ok(TICKET)), &hosted);
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&done, 0.0);
    let first = clock.view(&done, f64::from(CONSOLE_LINE_TYPING_MS) / 2.0, true);
    assert_eq!(first.lines.len(), 1);
    assert_eq!(first.lines[0].word, None);
    assert!(first.moving);
    let before_floor = clock.view(&done, f64::from(CONSOLE_SHORTEST_RUN_MS) - 1.0, true);
    assert!(before_floor.lines.iter().all(|line| line.word.is_some()));
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
    assert_eq!(view.lines.len(), 3);
    assert!(view.lines[2].failed);
    assert_eq!(view.error.as_deref(), Some("could not open the session"));

    let refused = report(Some(RejectReason::VersionMismatch));
    let versions = joining_script(JoiningFacts {
        ticket: Ok(()),
        probe: Some(Ok(&refused)),
        join_answer: None,
        state: &NetState::Idle,
    });
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
    let seat = joining_script(JoiningFacts {
        ticket: Ok(()),
        probe: Some(Ok(&full)),
        join_answer: None,
        state: &NetState::Idle,
    });
    assert_eq!(seat.steps[3].end, StepEnd::Failed);
}

#[test]
fn no_motion_shows_the_console_whole() {
    let hosted = NetState::Hosting {
        ticket: TICKET.to_owned(),
    };
    let done = hosting(Some(Ok(TICKET)), &hosted);
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&done, 0.0);
    let view = clock.view(&done, 0.0, false);
    assert!(view.lines.iter().all(|line| line.word.is_some()));
    assert!(view.lobby_due, "with motion off the lobby shows once the work is done");
    assert!(!view.moving);

    let waiting = hosting(None, &NetState::Idle);
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&waiting, 0.0);
    let view = clock.view(&waiting, 0.0, false);
    assert_eq!(view.lines.len(), 3);
    assert!(view.lines[2].typed.starts_with("SESSION"));
    assert_eq!(view.lines[2].word, None);
}

#[test]
fn a_skip_shows_the_end_state_at_once() {
    let hosted = NetState::Hosting {
        ticket: TICKET.to_owned(),
    };
    let done = hosting(Some(Ok(TICKET)), &hosted);
    let mut clock = ConsoleClock::new(0.0);
    clock.observe(&done, 0.0);
    clock.skip();
    assert!(clock.view(&done, 1.0, true).lobby_due);
}
