use replicant_proto::RejectReason;

use crate::event::NetState;
use crate::report::ConnectionReport;

const FAILED_WORD: &str = "FAILED";
const LOBBY_LABEL: &str = "LOBBY";
const LOBBY_WORD: &str = "READY";

/// Which wait the boot console covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleTask {
    Hosting,
    Joining,
}

/// Where one step of the wait stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepEnd {
    Running,
    Passed,
    Failed,
}

/// One console line: a label, and the status word it closes with once its step has ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleStep {
    pub label: &'static str,
    pub word: &'static str,
    pub end: StepEnd,
}

/// The lines of one wait in order, the last one closing the console, and the error text of a failed step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleScript {
    pub steps: Vec<ConsoleStep>,
    pub error: Option<String>,
}

/// What hosting has done so far: the two checks made before the session is asked for, the answer of the
/// host call (the ticket, or the error text) once it has come, and the session's state.
#[derive(Debug, Clone, Copy)]
pub struct HostingFacts<'a> {
    pub game_folder: Result<(), &'a str>,
    pub mod_files: Result<(), &'a str>,
    pub host_answer: Option<Result<&'a str, &'a str>>,
    pub state: &'a NetState,
}

/// What joining has done so far: the ticket's own check, the connection test's report (or its error text),
/// the join call's answer, and the session's state.
#[derive(Debug, Clone, Copy)]
pub struct JoiningFacts<'a> {
    pub ticket: Result<(), &'a str>,
    pub probe: Option<Result<&'a ConnectionReport, &'a str>>,
    pub join_answer: Option<Result<(), &'a str>>,
    pub state: &'a NetState,
}

enum Outcome<'a> {
    Running,
    Passed,
    Failed(&'a str),
}

impl ConsoleScript {
    pub fn failed(&self) -> bool {
        self.steps.iter().any(|step| step.end == StepEnd::Failed)
    }

    pub fn finished(&self) -> bool {
        self.steps.iter().all(|step| step.end == StepEnd::Passed)
    }
}

/// The hosting wait: the game is found, the mod's files are in place, the session is open, the ticket is
/// made.
pub fn hosting_script(facts: HostingFacts) -> ConsoleScript {
    let session_open = match (facts.host_answer, facts.state) {
        (Some(Err(text)), _) => Outcome::Failed(text),
        (Some(Ok(_)), _) | (None, NetState::Hosting { .. } | NetState::Connected { .. }) => {
            Outcome::Passed
        }
        _ => Outcome::Running,
    };
    let ticket_made = match (facts.host_answer, facts.state) {
        (Some(Ok(ticket)), _) if !ticket.is_empty() => Outcome::Passed,
        (_, NetState::Hosting { ticket }) if !ticket.is_empty() => Outcome::Passed,
        _ => Outcome::Running,
    };
    in_order(vec![
        ("GAME FOLDER", "FOUND", checked(facts.game_folder)),
        ("MOD FILES", "PRESENT", checked(facts.mod_files)),
        ("SESSION", "OPEN", session_open),
        ("TICKET", "ISSUED", ticket_made),
    ])
}

/// The joining wait: the ticket is read, the host is reached, the versions match, the seat is taken.
pub fn joining_script(facts: JoiningFacts) -> ConsoleScript {
    let rejected = match facts.probe {
        Some(Ok(report)) => report.rejected,
        _ => None,
    };
    let host_reached = match facts.probe {
        Some(Ok(report)) if report.reachable && (report.rtt.is_some() || rejected.is_some()) => {
            Outcome::Passed
        }
        Some(Ok(report)) => Outcome::Failed(&report.verdict),
        Some(Err(text)) => Outcome::Failed(text),
        None => Outcome::Running,
    };
    let versions_match = match (facts.probe, rejected) {
        (
            Some(Ok(report)),
            Some(
                RejectReason::BuildMismatch
                | RejectReason::VersionMismatch
                | RejectReason::Malformed,
            ),
        ) => Outcome::Failed(&report.verdict),
        (Some(Ok(_)), _) => Outcome::Passed,
        _ => Outcome::Running,
    };
    let seat_taken = match (facts.probe, rejected, facts.join_answer, facts.state) {
        (Some(Ok(report)), Some(_), _, _) => Outcome::Failed(&report.verdict),
        (_, _, Some(Err(text)), _) => Outcome::Failed(text),
        (_, _, Some(Ok(())), NetState::Connected { .. }) => Outcome::Passed,
        (_, _, Some(Ok(())), NetState::Failed(text)) => Outcome::Failed(text),
        _ => Outcome::Running,
    };
    in_order(vec![
        ("TICKET", "READ", checked(facts.ticket)),
        ("HOST", "REACHED", host_reached),
        ("VERSIONS", "MATCH", versions_match),
        ("SEAT", "TAKEN", seat_taken),
    ])
}

fn checked(check: Result<(), &str>) -> Outcome<'_> {
    match check {
        Ok(()) => Outcome::Passed,
        Err(text) => Outcome::Failed(text),
    }
}

fn in_order(lines: Vec<(&'static str, &'static str, Outcome)>) -> ConsoleScript {
    let mut steps = Vec::with_capacity(lines.len() + 1);
    let mut error = None;
    // A step counts only once every step before it has passed: the lines are one sequence.
    let mut open = true;
    for (label, word, outcome) in lines {
        let end = match outcome {
            _ if !open => StepEnd::Running,
            Outcome::Passed => StepEnd::Passed,
            Outcome::Running => StepEnd::Running,
            Outcome::Failed(text) => {
                error = Some(text.to_owned());
                StepEnd::Failed
            }
        };
        open = end == StepEnd::Passed;
        let word = if end == StepEnd::Failed {
            FAILED_WORD
        } else {
            word
        };
        steps.push(ConsoleStep { label, word, end });
    }
    steps.push(ConsoleStep {
        label: LOBBY_LABEL,
        word: LOBBY_WORD,
        end: if open {
            StepEnd::Passed
        } else {
            StepEnd::Running
        },
    });
    ConsoleScript { steps, error }
}
