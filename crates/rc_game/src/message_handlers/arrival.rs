use crate::script_functions::script_function;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandlerArrivalFacts<'a> {
    pub enabled: bool,
    pub local_is_guest: bool,
    pub local_in_play: bool,
    pub local_phase: Option<u16>,
    pub call_phase: u16,
    pub phase_label: &'a str,
    pub queue_is_empty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerDrop {
    Disabled,
    NotTheGuest,
    NotInPlay,
    WrongPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerArrival {
    Drop(HandlerDrop),
    RunNow,
    Park,
    ForcedDrain,
}

impl HandlerDrop {
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Disabled => "shared story events are off",
            Self::NotTheGuest => "only the host's handler runs are followed",
            Self::NotInPlay => "this game is not in play",
            Self::WrongPhase => "this game is in another phase",
        }
    }
}

pub fn handler_arrival(facts: &HandlerArrivalFacts, name: &str) -> HandlerArrival {
    if !facts.enabled {
        return HandlerArrival::Drop(HandlerDrop::Disabled);
    }
    if !facts.local_is_guest {
        return HandlerArrival::Drop(HandlerDrop::NotTheGuest);
    }
    if !facts.local_in_play {
        return HandlerArrival::Drop(HandlerDrop::NotInPlay);
    }
    if facts.local_phase != Some(facts.call_phase) {
        return HandlerArrival::Drop(HandlerDrop::WrongPhase);
    }
    let starts_scene =
        script_function(facts.phase_label, name).is_some_and(|function| function.starts_scene);
    if starts_scene {
        HandlerArrival::ForcedDrain
    } else if facts.queue_is_empty {
        HandlerArrival::RunNow
    } else {
        HandlerArrival::Park
    }
}
