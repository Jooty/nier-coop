use crate::script_functions::{FunctionsRan, is_callee_of, script_function};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionArrivalFacts<'a> {
    pub enabled: bool,
    pub local_in_play: bool,
    pub local_phase: Option<u16>,
    pub call_phase: u16,
    pub phase_label: &'a str,
    pub queue_is_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionDrop {
    Disabled,
    NotInPlay,
    WrongPhase,
    NotASharedFunction,
    AlreadyRan,
    CalleeOf(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionArrival {
    Drop(FunctionDrop),
    RunNow,
    Park,
    ForcedDrain,
}

impl FunctionDrop {
    pub fn describe(&self) -> String {
        match self {
            Self::Disabled => "shared story events are off".to_owned(),
            Self::NotInPlay => "this game is not in play".to_owned(),
            Self::WrongPhase => "this game is in another phase".to_owned(),
            Self::NotASharedFunction => "not a function of a shared event".to_owned(),
            Self::AlreadyRan => "it ran here already".to_owned(),
            Self::CalleeOf(caller) => {
                format!("it is called by {caller}, which ran or is queued here")
            }
        }
    }
}

pub fn function_arrival(
    facts: &FunctionArrivalFacts,
    name: &str,
    ran: &FunctionsRan,
    queued: &[String],
) -> FunctionArrival {
    if !facts.enabled {
        return FunctionArrival::Drop(FunctionDrop::Disabled);
    }
    if !facts.local_in_play {
        return FunctionArrival::Drop(FunctionDrop::NotInPlay);
    }
    if facts.local_phase != Some(facts.call_phase) {
        return FunctionArrival::Drop(FunctionDrop::WrongPhase);
    }
    let Some(function) = script_function(facts.phase_label, name) else {
        return FunctionArrival::Drop(FunctionDrop::NotASharedFunction);
    };
    if let Some(drop) = judge_function(facts.phase_label, facts.call_phase, name, ran, queued) {
        return FunctionArrival::Drop(drop);
    }
    if function.starts_scene {
        FunctionArrival::ForcedDrain
    } else if facts.queue_is_empty {
        FunctionArrival::RunNow
    } else {
        FunctionArrival::Park
    }
}

pub fn judge_function(
    phase_label: &str,
    phase: u16,
    name: &str,
    ran: &FunctionsRan,
    queued: &[String],
) -> Option<FunctionDrop> {
    if ran.contains(phase, name) {
        return Some(FunctionDrop::AlreadyRan);
    }
    ran.names_in(phase)
        .chain(queued.iter().map(String::as_str))
        .find(|caller| is_callee_of(phase_label, name, caller))
        .map(|caller| FunctionDrop::CalleeOf(caller.to_owned()))
}
