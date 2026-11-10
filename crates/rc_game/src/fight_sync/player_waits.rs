use crate::fight_sync::PLAYER_WAIT_BEHIND_STREAM_GRACE_MS;

/// A waiting script native that holds the script slot until this game's own player reaches a
/// state (back on its feet, on the ground).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerWaitNative {
    DefaultStatusReserve,
    OnGround,
}

impl PlayerWaitNative {
    pub const ALL: [Self; 2] = [Self::DefaultStatusReserve, Self::OnGround];

    pub const fn name(self) -> &'static str {
        match self {
            Self::DefaultStatusReserve => "_WaitPlayerDefaultStatusReserve",
            Self::OnGround => "_WaitPlayerOnGround",
        }
    }

    /// The function the script step polls while this native waits (manager + 0xD8 in, a byte
    /// out that is non-zero while it waits).
    pub const fn poll_rva(self) -> u32 {
        match self {
            Self::DefaultStatusReserve => 0x70_C310,
            Self::OnGround => 0x70_C4C0,
        }
    }
}

/// The guest ends its own player wait once a later host stream entry has waited behind the
/// script slot past `PLAYER_WAIT_BEHIND_STREAM_GRACE_MS`: the host's copy of the handler is over.
pub fn player_wait_ends(still_waiting: bool, entry_behind_ms: Option<u64>) -> bool {
    still_waiting
        && entry_behind_ms.is_some_and(|behind_ms| behind_ms > PLAYER_WAIT_BEHIND_STREAM_GRACE_MS)
}

pub fn format_player_wait_release(
    native: PlayerWaitNative,
    handler: &str,
    entry_behind_ms: u64,
) -> String {
    format!(
        "player wait: ended your wait in {handler} (the host moved on, {}, a host entry waited {entry_behind_ms} ms behind it)",
        native.name()
    )
}
