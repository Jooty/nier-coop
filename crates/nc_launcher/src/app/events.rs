use crate::lobby::body_label;
use crate::net::{LauncherEvent, MemberChange};

pub fn describe_event(event: &LauncherEvent) -> String {
    match event {
        LauncherEvent::Hosting { .. } => "hosting, waiting for a player".to_owned(),
        LauncherEvent::Joining => "joining the host...".to_owned(),
        LauncherEvent::MemberJoined { name, .. } => format!("{name} connected"),
        LauncherEvent::MemberLeft { detail, .. } => format!("the other player left ({detail})"),
        LauncherEvent::PeerRefused { reason } => format!("refused a player: {reason:?}"),
        LauncherEvent::Rejected { reason } => format!("the host said no: {reason:?}"),
        LauncherEvent::GameLinked => "the game connected to the launcher".to_owned(),
        LauncherEvent::GameUnlinked { detail } => {
            format!("your game closed its link to the launcher ({detail})")
        }
        LauncherEvent::OwnGameReadiness { readiness } => {
            format!("readiness: your game {readiness:?}")
        }
        LauncherEvent::GameChangedSettings { .. } => "the game changed a setting".to_owned(),
        LauncherEvent::SessionOptionsChanged { .. } => "the host options changed".to_owned(),
        LauncherEvent::MemberChanged { change, .. } => match change {
            MemberChange::Readiness(readiness) => {
                format!("readiness: the other game {readiness:?}")
            }
            MemberChange::Lobby(ready) => format!(
                "lobby: the other player is {} (mode {})",
                if ready.ready { "ready" } else { "not ready" },
                ready.mode
            ),
            MemberChange::Body(body) => {
                format!("lobby: the guest picked {}", body_label(*body))
            }
        },
        LauncherEvent::LobbyStartReceived { mode } => {
            format!("lobby: the host started (mode {mode})")
        }
        LauncherEvent::GameSaved { succeeded } => {
            if *succeeded {
                "the game saved".to_owned()
            } else {
                "the game tried to save and the write failed".to_owned()
            }
        }
        LauncherEvent::CampaignCardReceived { card } => {
            format!("campaign: the host sent the card for {}", card.host.name)
        }
        LauncherEvent::SeatReceived { transfer, seat, .. } => {
            format!(
                "save transfer {transfer}: received whole ({} bytes)",
                seat.len()
            )
        }
        LauncherEvent::SeatDelivered { transfer, .. } => {
            format!("save transfer {transfer}: the other side has it whole")
        }
        LauncherEvent::SeatTransferRefused {
            transfer,
            reason,
            by_peer,
            ..
        } => {
            let who = if *by_peer {
                "the other side"
            } else {
                "this side"
            };
            format!("save transfer {transfer}: refused by {who} ({reason:?})")
        }
        LauncherEvent::SeatProgress {
            transfer,
            parts_done,
            parts,
            incoming,
            ..
        } => {
            let way = if *incoming { "receiving" } else { "sending" };
            format!("save transfer {transfer}: {way} part {parts_done} of {parts}")
        }
        LauncherEvent::MessageRefused { kind, reason } => {
            format!("dropped a {kind} message, it was not sent: {reason}")
        }
        LauncherEvent::Failed { message } => format!("network problem: {message}"),
        LauncherEvent::Stopped => "stopped".to_owned(),
    }
}
