use crate::net::LobbyReady;

pub fn start_enabled(
    connected: bool,
    is_guest: bool,
    already_started: bool,
    own: LobbyReady,
    partner: Option<LobbyReady>,
) -> bool {
    connected
        && !is_guest
        && !already_started
        && own.ready
        && partner.is_some_and(|partner| partner.ready)
}

pub fn guest_accepts_start(
    connected: bool,
    is_guest: bool,
    already_started: bool,
    own: LobbyReady,
) -> bool {
    connected && is_guest && !already_started && own.ready
}
