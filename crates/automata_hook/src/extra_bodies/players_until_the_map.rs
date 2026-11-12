use automata_game::partner::{OtherPlayer, SessionMode};
use automata_proto::PlayerState;

use crate::net::AutomataNet;
use nc_hook::net::NetThread;

/// One other player with the newest state heard from it, kept only while it is fresh.
#[derive(Debug, Clone, Copy)]
pub struct FurtherPlayer {
    pub roster: OtherPlayer,
    pub fresh_state: Option<PlayerState>,
}

/// The session's mode. Always `Campaign` until the modes land (G1): then it reads the mode the
/// session was made with.
pub fn session_mode_until_modes_land() -> SessionMode {
    SessionMode::Campaign
}

/// Every other player of the session, lowest slot included (the roster leaves that one to the
/// partner machine). Answers "no further players" until the players map lands (GM, G2): then it
/// maps `NetShared::others()`, each `Player` giving its slot, `watch.character`, in play as
/// `latest` without `load.without_body`, the age of `latest`, and `latest` while it is younger
/// than `REMOTE_STATE_STALE_MS`.
pub fn other_players_until_the_players_map_lands(
    _net: &NetThread<AutomataNet>,
) -> Vec<FurtherPlayer> {
    Vec::new()
}
