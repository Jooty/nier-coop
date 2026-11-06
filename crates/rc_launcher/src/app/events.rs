use crate::net::LauncherEvent;

pub fn describe_event(event: &LauncherEvent) -> String {
    match event {
        LauncherEvent::Hosting { .. } => "hosting, waiting for a player".to_owned(),
        LauncherEvent::Joining => "joining the host...".to_owned(),
        LauncherEvent::PeerJoined { name } => format!("{name} connected"),
        LauncherEvent::PeerLeft => "the other player left".to_owned(),
        LauncherEvent::PeerRefused { reason } => format!("refused a player: {reason:?}"),
        LauncherEvent::Rejected { reason } => format!("the host said no: {reason:?}"),
        LauncherEvent::GameLinked => "the game connected to the launcher".to_owned(),
        LauncherEvent::GameUnlinked => "the game disconnected".to_owned(),
        LauncherEvent::OwnGameReadiness { readiness } => {
            format!("readiness: your game {readiness:?}")
        }
        LauncherEvent::PartnerGameReadiness { readiness } => {
            format!("readiness: the other game {readiness:?}")
        }
        LauncherEvent::GameChangedSettings { .. } => "the game changed a setting".to_owned(),
        LauncherEvent::SessionOptionsChanged { .. } => "the host options changed".to_owned(),
        LauncherEvent::LobbyPartnerReady { ready } => format!(
            "lobby: the other player is {} (mode {})",
            if ready.ready { "ready" } else { "not ready" },
            ready.mode
        ),
        LauncherEvent::LobbyStartReceived { mode } => {
            format!("lobby: the host started (mode {mode})")
        }
        LauncherEvent::Failed { message } => format!("network problem: {message}"),
        LauncherEvent::Stopped => "stopped".to_owned(),
    }
}
