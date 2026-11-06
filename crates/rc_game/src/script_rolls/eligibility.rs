use crate::script_rolls::{GATHERING_ROLL_FUNCTIONS, ROLL_ALLOW_LIST};
use crate::story_calls::name_hash;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RollFacts {
    pub function_hash: u64,
    pub phase_hash: Option<u64>,
    pub relayed_story_call: bool,
    pub from_talk_tag: bool,
}

pub fn roll_is_shared(facts: &RollFacts) -> bool {
    if facts.from_talk_tag || is_gathering(facts.function_hash) {
        return false;
    }
    facts.relayed_story_call || is_allow_listed(facts)
}

fn is_gathering(function_hash: u64) -> bool {
    GATHERING_ROLL_FUNCTIONS
        .iter()
        .any(|name| name_hash(name) == function_hash)
}

fn is_allow_listed(facts: &RollFacts) -> bool {
    ROLL_ALLOW_LIST.iter().any(|&(phase, function)| {
        name_hash(function) == facts.function_hash
            && (phase.is_empty() || facts.phase_hash == Some(name_hash(phase)))
    })
}
