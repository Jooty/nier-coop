use crate::story_calls::name_hash;

/// A talk-tag callback that raises a script global counting the talks held; the count decides a
/// story step, so the host runs each such callback once for either player and the guest replays it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CountedTalkCallback {
    pub pack: &'static str,
    pub function: &'static str,
    pub count_global: &'static str,
}

pub const COUNTED_TALK_CALLBACKS: [CountedTalkCallback; 2] = [
    CountedTalkCallback {
        pack: "A_DESERT_TOWN_01",
        function: "Count_Talk_Num",
        count_global: "Count_Num",
    },
    CountedTalkCallback {
        pack: "A_SEASIDE_TOWN_01",
        function: "SeaSide_NPC_Talk_End",
        count_global: "NPC_Talk_Count",
    },
];

impl CountedTalkCallback {
    pub fn is(&self, pack_hash: u64, function_hash: u64) -> bool {
        name_hash(self.pack) == pack_hash && name_hash(self.function) == function_hash
    }
}

pub fn counted_talk_callback(
    pack_hash: u64,
    function_hash: u64,
) -> Option<&'static CountedTalkCallback> {
    COUNTED_TALK_CALLBACKS
        .iter()
        .find(|callback| callback.is(pack_hash, function_hash))
}

/// Whether a popped call stays with the player whose talk queued it: every talk-tag callback
/// does, except a counted one, whose count is the host's.
pub fn talk_keeps_the_call(
    queued_by_talk_tag: bool,
    pack_hash: Option<u64>,
    function_hash: u64,
) -> bool {
    queued_by_talk_tag
        && !pack_hash.is_some_and(|pack| counted_talk_callback(pack, function_hash).is_some())
}
