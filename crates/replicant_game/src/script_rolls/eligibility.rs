use crate::script_rolls::{ANY_PACK_HASH, GATHERING_ROLL_FUNCTION_HASHES, ROLL_ALLOW_LIST};

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
    GATHERING_ROLL_FUNCTION_HASHES.contains(&function_hash)
}

fn is_allow_listed(facts: &RollFacts) -> bool {
    ROLL_ALLOW_LIST.iter().any(|&(pack, function)| {
        function == facts.function_hash && (pack == ANY_PACK_HASH || facts.phase_hash == Some(pack))
    })
}
