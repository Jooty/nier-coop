use std::sync::LazyLock;

use crate::rule_rooms::{RULE_ROOMS, RuleAct, RuleRoom};
use crate::story_calls::name_hash;

static HOST_RUN_CALL_HASHES: LazyLock<Vec<(u64, u64)>> = LazyLock::new(|| {
    let mut hashes = Vec::new();
    for room in &RULE_ROOMS {
        let pack = name_hash(room.pack);
        for start in room.starts {
            hashes.push((pack, name_hash(start)));
        }
    }
    hashes
});

/// What a popped function is to a rule room of its pack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleRoomCall {
    Start,
    Break(RuleAct),
    Arm,
    Restart,
    Clear,
}

pub fn rule_room_call(pack: &str, function: &str) -> Option<(&'static RuleRoom, RuleRoomCall)> {
    RULE_ROOMS
        .iter()
        .filter(|room| room.pack == pack)
        .find_map(|room| {
            let call = if room.starts.contains(&function) {
                RuleRoomCall::Start
            } else if let Some(act) = room.act_of(function) {
                RuleRoomCall::Break(act)
            } else if room.arms.contains(&function) {
                RuleRoomCall::Arm
            } else if room.restart == function {
                RuleRoomCall::Restart
            } else if room.clear == function {
                RuleRoomCall::Clear
            } else {
                return None;
            };
            Some((room, call))
        })
}

/// A room's start is the host's call that every game in the room runs: the story relay treats it
/// as a story call. A break is never relayed: it belongs to the game of the player who broke it.
pub fn is_host_run_rule_room_call(pack_hash: u64, function_hash: u64) -> bool {
    HOST_RUN_CALL_HASHES.contains(&(pack_hash, function_hash))
}
