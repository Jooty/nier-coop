use automata_game::mode::SessionMode;
use automata_game::partner::{OtherPlayer, REMOTE_STATE_STALE_MS};
use automata_proto::PlayerState;

use crate::net::{AutomataNet, SessionModeExt};
use nt_hook::net::NetThread;

/// One other player with the newest state heard from it, kept only while it is fresh.
#[derive(Debug, Clone, Copy)]
pub struct FurtherPlayer {
    pub roster: OtherPlayer,
    pub fresh_state: Option<PlayerState>,
}

/// The session's mode, read from the rules the host's welcome set.
pub fn session_mode_of(net: &NetThread<AutomataNet>) -> SessionMode {
    net.update(|shared| shared.session_mode())
}

/// Every other player of the session, lowest slot included (the roster leaves that one to the
/// partner machine). Each gives its slot, `watch.character`, in play as `latest` without
/// `load.without_body`, the age of `latest`, and `latest` while it is younger than
/// `REMOTE_STATE_STALE_MS`.
pub fn other_players_of(net: &NetThread<AutomataNet>) -> Vec<FurtherPlayer> {
    net.update(|shared| {
        shared
            .others()
            .map(|player| {
                let latest = player.latest.as_ref();
                let age_ms = latest.map(|remote| remote.age().as_millis() as u64);
                FurtherPlayer {
                    roster: OtherPlayer {
                        slot: player.slot,
                        character: player.watch.character,
                        in_play: latest.is_some_and(|remote| !remote.state.load.without_body),
                        newest_state_age_ms: age_ms,
                    },
                    fresh_state: latest
                        .filter(|_| age_ms.is_some_and(|age| age <= REMOTE_STATE_STALE_MS))
                        .map(|remote| remote.state),
                }
            })
            .collect()
    })
}
