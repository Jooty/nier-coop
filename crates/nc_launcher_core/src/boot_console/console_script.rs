use replicant_proto::RejectReason;

use crate::event::NetState;
use crate::report::{ConnectionReport, PathKind};

const FAILED_WORD: &str = "FAILED";
const LOBBY_LABEL: &str = "LOBBY";
const LOBBY_WORD: &str = "READY";
const TICKET_START_CHARACTERS: usize = 12;
const ELLIPSIS: char = '\u{2026}';

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

/// A fact under a step: what the step found or set up, told once the step has passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleDetail {
    pub label: &'static str,
    pub value: String,
}

/// One console step: a label, the value it stands for when the launcher holds one, the status word it closes
/// with once it has ended, and the facts under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleStep {
    pub label: &'static str,
    pub value: Option<String>,
    pub word: &'static str,
    pub end: StepEnd,
    pub details: Vec<ConsoleDetail>,
}

/// The steps of one wait in order, the last one closing the console, and the error text of a failed step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleScript {
    pub steps: Vec<ConsoleStep>,
    pub error: Option<String>,
}

/// The save a hosted saved game continues: its slot counted from 0, its level and its play time as text.
#[derive(Debug, Clone, Copy)]
pub struct ConsoleSave<'a> {
    pub slot: u8,
    pub level: i32,
    pub play_time: &'a str,
}

/// The launcher's own facts about the game and the mod, held before any network work.
#[derive(Debug, Clone, Copy)]
pub struct ConsoleGameFacts<'a> {
    pub protocol: u16,
    pub installed_build: Option<&'a str>,
    pub waiting_update: Option<&'a str>,
    pub player_name: &'a str,
}

/// What hosting has done so far: the two checks made before the session is asked for (the folder's path or
/// the error text), the answer of the host call (the ticket or the error text) once it has come, the
/// session's state, and the facts the session is opened with. `saved_game` is `None` when the kind of game
/// is not known, `Some(None)` for a new game.
#[derive(Debug, Clone, Copy)]
pub struct HostingFacts<'a> {
    pub game_folder: Result<&'a str, &'a str>,
    pub executable: Option<&'a str>,
    pub mod_files: Result<(), &'a str>,
    pub hook_file: &'a str,
    pub game: ConsoleGameFacts<'a>,
    pub saved_game: Option<Option<ConsoleSave<'a>>>,
    pub seats: u8,
    pub enemy_health_percent: u16,
    pub enemy_damage_percent: u16,
    pub host_answer: Option<Result<&'a str, &'a str>>,
    pub state: &'a NetState,
}

/// What joining has done so far: the ticket and its own check, the connection test's report (or its error
/// text), the join call's answer, the session's state, and the seat the session gave this player.
#[derive(Debug, Clone, Copy)]
pub struct JoiningFacts<'a> {
    pub ticket_text: &'a str,
    pub ticket: Result<(), &'a str>,
    pub game: ConsoleGameFacts<'a>,
    pub probe: Option<Result<&'a ConnectionReport, &'a str>>,
    pub join_answer: Option<Result<(), &'a str>>,
    pub state: &'a NetState,
    pub own_slot: Option<u8>,
}

enum Outcome<'a> {
    Running,
    Passed,
    Failed(&'a str),
}

struct Planned<'a> {
    label: &'static str,
    value: Option<String>,
    word: &'static str,
    outcome: Outcome<'a>,
    details: Vec<ConsoleDetail>,
}

impl ConsoleScript {
    pub fn failed(&self) -> bool {
        self.steps.iter().any(|step| step.end == StepEnd::Failed)
    }

    pub fn finished(&self) -> bool {
        self.steps.iter().all(|step| step.end == StepEnd::Passed)
    }

    /// The steps that count toward the meter: every step but the closing one.
    pub fn counted_steps(&self) -> usize {
        self.steps.len().saturating_sub(1)
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
    let ticket = match (facts.host_answer, facts.state) {
        (Some(Ok(ticket)), _) if !ticket.is_empty() => Some(ticket),
        (_, NetState::Hosting { ticket }) if !ticket.is_empty() => Some(ticket.as_str()),
        _ => None,
    };
    let folder = Planned {
        label: "GAME FOLDER",
        value: facts.game_folder.ok().map(str::to_owned),
        word: "FOUND",
        outcome: checked(facts.game_folder.map(|_| ())),
        details: details([("EXECUTABLE", facts.executable.map(str::to_owned))]),
    };
    let mod_files = Planned {
        label: "MOD FILES",
        value: facts.game.installed_build.map(build_value),
        word: "PRESENT",
        outcome: checked(facts.mod_files),
        details: details([
            ("FILE", Some(facts.hook_file.to_owned())),
            ("UPDATE", waiting_update(facts.game)),
            ("PROTOCOL", Some(facts.game.protocol.to_string())),
        ]),
    };
    let save = facts.saved_game.flatten();
    let session = Planned {
        label: "SESSION",
        value: facts.saved_game.map(|saved| {
            if saved.is_some() {
                "SAVED GAME".to_owned()
            } else {
                "NEW GAME".to_owned()
            }
        }),
        word: "OPEN",
        outcome: session_open,
        details: details([
            ("PLAYER", Some(facts.game.player_name.to_owned())),
            (
                "SAVE",
                save.map(|save| format!("SLOT {:02}", u16::from(save.slot) + 1)),
            ),
            ("LEVEL", save.map(|save| save.level.to_string())),
            ("PLAY TIME", save.map(|save| save.play_time.to_owned())),
            ("SEATS", Some(facts.seats.to_string())),
            (
                "ENEMY HEALTH",
                Some(per_extra_player(facts.enemy_health_percent)),
            ),
            (
                "ENEMY DAMAGE",
                Some(per_extra_player(facts.enemy_damage_percent)),
            ),
        ]),
    };
    let ticket_line = Planned {
        label: "TICKET",
        value: ticket.map(ticket_length),
        word: "ISSUED",
        outcome: if ticket.is_some() {
            Outcome::Passed
        } else {
            Outcome::Running
        },
        details: details([("BEGINS", ticket.map(ticket_start))]),
    };
    in_order(vec![folder, mod_files, session, ticket_line])
}

/// The joining wait: the ticket is read, the host is reached, the versions match, the seat is taken.
pub fn joining_script(facts: JoiningFacts) -> ConsoleScript {
    let report = match facts.probe {
        Some(Ok(report)) => Some(report),
        _ => None,
    };
    let rejected = report.and_then(|report| report.rejected);
    let host_reached = match facts.probe {
        Some(Ok(report)) if report.reachable && (report.rtt.is_some() || rejected.is_some()) => {
            Outcome::Passed
        }
        Some(Ok(report)) => Outcome::Failed(&report.verdict),
        Some(Err(text)) => Outcome::Failed(text),
        None => Outcome::Running,
    };
    let versions_match = match (report, rejected) {
        (
            Some(report),
            Some(
                RejectReason::BuildMismatch
                | RejectReason::VersionMismatch
                | RejectReason::Malformed,
            ),
        ) => Outcome::Failed(&report.verdict),
        (Some(_), _) => Outcome::Passed,
        _ => Outcome::Running,
    };
    let seat_taken = match (report, rejected, facts.join_answer, facts.state) {
        (Some(report), Some(_), _, _) => Outcome::Failed(&report.verdict),
        (_, _, Some(Err(text)), _) => Outcome::Failed(text),
        (_, _, Some(Ok(())), NetState::Connected { .. }) => Outcome::Passed,
        (_, _, Some(Ok(())), NetState::Failed(text)) => Outcome::Failed(text),
        _ => Outcome::Running,
    };
    let ticket_text = facts.ticket_text.trim();
    let ticket_held = (!ticket_text.is_empty()).then_some(ticket_text);
    let ticket = Planned {
        label: "TICKET",
        value: ticket_held.map(ticket_length),
        word: "READ",
        outcome: checked(facts.ticket),
        details: details([("BEGINS", ticket_held.map(ticket_start))]),
    };
    let host_name = report.and_then(|report| report.host_name.clone());
    let rtt = report.and_then(|report| report.rtt);
    let host = Planned {
        label: "HOST",
        value: host_name.clone(),
        word: "REACHED",
        outcome: host_reached,
        details: details([
            (
                "PATH",
                report.and_then(|report| match report.path {
                    PathKind::Direct => Some("DIRECT".to_owned()),
                    PathKind::Relay => Some("RELAYED".to_owned()),
                    PathKind::Unknown => None,
                }),
            ),
            (
                "ROUND TRIP",
                rtt.map(|rtt| {
                    format!(
                        "{} MS  ({} TO {})",
                        rtt.avg.as_millis(),
                        rtt.min.as_millis(),
                        rtt.max.as_millis()
                    )
                }),
            ),
            (
                "LOSS",
                report
                    .filter(|report| report.rtt.is_some())
                    .map(|report| format!("{:.0} %", report.loss_percent)),
            ),
        ]),
    };
    let versions = Planned {
        label: "VERSIONS",
        value: Some(format!("PROTOCOL {}", facts.game.protocol)),
        word: "MATCH",
        outcome: versions_match,
        details: details([
            ("MOD", facts.game.installed_build.map(build_value)),
            ("UPDATE", waiting_update(facts.game)),
        ]),
    };
    let peer_name = match facts.state {
        NetState::Connected { peer_name } => Some(peer_name.clone()),
        _ => None,
    };
    let seat = Planned {
        label: "SEAT",
        value: facts
            .own_slot
            .filter(|_| peer_name.is_some())
            .map(|slot| format!("{:02}", u16::from(slot) + 1)),
        word: "TAKEN",
        outcome: seat_taken,
        details: details([
            ("PLAYER", Some(facts.game.player_name.to_owned())),
            // The host's name is told here only when the connection test did not already tell it.
            ("HOST NAME", peer_name.filter(|_| host_name.is_none())),
        ]),
    };
    in_order(vec![ticket, host, versions, seat])
}

fn checked(check: Result<(), &str>) -> Outcome<'_> {
    match check {
        Ok(()) => Outcome::Passed,
        Err(text) => Outcome::Failed(text),
    }
}

fn details<const N: usize>(facts: [(&'static str, Option<String>); N]) -> Vec<ConsoleDetail> {
    facts
        .into_iter()
        .filter_map(|(label, value)| {
            value
                .filter(|value| !value.trim().is_empty())
                .map(|value| ConsoleDetail { label, value })
        })
        .collect()
}

fn waiting_update(game: ConsoleGameFacts) -> Option<String> {
    game.waiting_update
        .map(|build| format!("{} WAITING", build_value(build)))
}

fn build_value(build: &str) -> String {
    format!("BUILD {build}")
}

fn per_extra_player(percent: u16) -> String {
    format!("+{percent} % PER EXTRA PLAYER")
}

fn ticket_length(ticket: &str) -> String {
    format!("{} CHARACTERS", ticket.chars().count())
}

fn ticket_start(ticket: &str) -> String {
    let mut start: String = ticket.chars().take(TICKET_START_CHARACTERS).collect();
    if ticket.chars().count() > TICKET_START_CHARACTERS {
        start.push(ELLIPSIS);
    }
    start
}

fn in_order(planned: Vec<Planned>) -> ConsoleScript {
    let mut steps = Vec::with_capacity(planned.len() + 1);
    let mut error = None;
    // A step counts only once every step before it has passed: the lines are one sequence.
    let mut open = true;
    for line in planned {
        let end = match line.outcome {
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
            line.word
        };
        let details = if end == StepEnd::Passed {
            line.details
        } else {
            Vec::new()
        };
        steps.push(ConsoleStep {
            label: line.label,
            value: line.value,
            word,
            end,
            details,
        });
    }
    steps.push(ConsoleStep {
        label: LOBBY_LABEL,
        value: None,
        word: LOBBY_WORD,
        end: if open {
            StepEnd::Passed
        } else {
            StepEnd::Running
        },
        details: Vec::new(),
    });
    ConsoleScript { steps, error }
}
