use crate::lobby::body_label;
use crate::net::LauncherEvent;

pub fn describe_event(event: &LauncherEvent) -> String {
    match event {
        LauncherEvent::Hosting { .. } => "hosting, waiting for a player".to_owned(),
        LauncherEvent::Joining => "joining the host...".to_owned(),
        LauncherEvent::PeerJoined { name } => format!("{name} connected"),
        LauncherEvent::PeerLeft { detail } => format!("the other player left ({detail})"),
        LauncherEvent::PeerRefused { reason } => format!("refused a player: {reason:?}"),
        LauncherEvent::Rejected { reason } => format!("the host said no: {reason:?}"),
        LauncherEvent::GameLinked => "the game connected to the launcher".to_owned(),
        LauncherEvent::GameUnlinked { detail } => {
            format!("your game closed its link to the launcher ({detail})")
        }
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
        LauncherEvent::LobbyPartnerBody { body } => {
            format!("lobby: the guest picked {}", body_label(*body))
        }
        LauncherEvent::LobbyPartnerCharacter { index } => {
            format!("lobby: the other player picked character number {index}")
        }
        LauncherEvent::LobbyStartReceived { mode } => {
            format!("lobby: the host started (mode {mode})")
        }
        LauncherEvent::MessageRefused { kind, reason } => {
            format!("dropped a {kind} message, it was not sent: {reason}")
        }
        LauncherEvent::Failed { message } => format!("network problem: {message}"),
        LauncherEvent::Stopped => "stopped".to_owned(),
    }
}
