use nc_hook::diagnostics::log_line;
use nc_hook::net::{LinkAccess, NetGame, PlayerOf, log_readiness, net_clock_ms};
use nc_net::Channel;
use replicant_game::PHASE_TABLE;
use replicant_game::carts::partner_wants_to_ride_text;
use replicant_game::readiness::HEARTBEAT_INTERVAL_MS;
use replicant_proto::{GameReadiness, Message, PartnerNotice, PlayerState};

use crate::event_log::post_event;
use crate::net::NetState;
use crate::net::link_state::ReplicantLinkState;
use crate::net::local_snapshot::LocalSnapshot;
use crate::net::shared::RemotePlayer;

#[derive(Debug, Default)]
pub struct ReplicantNet {
    readiness_sent: Option<(GameReadiness, u64)>,
}

impl NetGame for ReplicantNet {
    type Message = Message;
    type Snapshot = LocalSnapshot;
    type PeerState = PlayerState;
    type PlayerWatch = ();
    type Extras = ReplicantLinkState;

    fn kind_name(message: &Message) -> &'static str {
        message.kind_name()
    }

    fn player_in_play(_watch: &()) -> bool {
        true
    }

    fn peer_in_play(extras: &ReplicantLinkState) -> bool {
        extras.partner.is_in_play()
    }

    fn snapshot_message(&mut self, snapshot: &LocalSnapshot, tick: u32) -> Message {
        Message::State(snapshot.to_player_state(tick))
    }

    fn on_message(&mut self, link: &LinkAccess<'_, Self>, from: u8, message: Message) {
        match message {
            Message::Readiness(readiness) => self.on_partner_readiness(link, readiness),
            Message::Notice(notice) => on_partner_notice(link, notice),
            Message::State(state) => on_partner_state(link, state),
            Message::SessionOptions(options) => link.shared().options = options.clamped(),
            Message::Settings(settings) => link.shared().pending_settings = Some(settings),
            Message::PathInfo(info) => {
                let mut shared = link.shared();
                shared.rtt = Some(std::time::Duration::from_micros(u64::from(info.rtt_micros)));
                shared.direct_path = Some(info.direct);
            }
            Message::QuitGame => {
                log_line("net: the launcher asked this game to quit");
                link.shared().quit_requested = true;
            }
            relayed if relayed.is_relayed() => {
                if !link.shared().partner.is_in_play() {
                    return;
                }
                link.push_relayed(from, relayed);
            }
            _ => {}
        }
    }

    fn on_player_joined(&mut self, extras: &mut ReplicantLinkState, _player: &mut PlayerOf<Self>) {
        extras.partner.partner_connected(net_clock_ms());
    }

    fn on_player_left(&mut self, extras: &mut ReplicantLinkState, _slot: u8) {
        self.on_peer_cleared(extras);
    }

    fn on_peer_cleared(&mut self, extras: &mut ReplicantLinkState) {
        extras.options = replicant_proto::SessionOptions::default();
        extras.partner.partner_gone();
    }

    fn on_launcher_linked(&mut self) {
        self.readiness_sent = None;
    }

    fn on_idle(
        &mut self,
        link: &LinkAccess<'_, Self>,
        send: &mut dyn FnMut(&Message, Channel) -> bool,
    ) {
        assume_silent_partner_in_play(link);
        self.send_readiness(link, send);
    }

    fn announce(&self, text: &str) {
        post_event(text);
    }
}

impl ReplicantNet {
    fn on_partner_readiness(&mut self, link: &LinkAccess<'_, Self>, readiness: GameReadiness) {
        log_readiness(&format!("readiness: partner {readiness:?}"));
        let mut shared = link.shared();
        let was_in_play = shared.partner.is_in_play();
        let before = shared.partner.readiness();
        let just_closed = shared.partner.partner_reported(readiness);
        let still_in_play = shared.partner.is_in_play();
        let name = shared.peer_name.clone();
        if was_in_play != still_in_play {
            shared.remote = None;
        }
        drop(shared);
        if let Some(name) = name {
            if just_closed {
                log_readiness("readiness: partner game closed");
                post_event(&format!("{name}'s game closed"));
            } else if still_in_play && !was_in_play {
                post_event(&format!("{name}'s game is in play"));
            } else if readiness == GameReadiness::Loading && before != GameReadiness::Loading {
                post_event(&format!("{name}'s game is loading"));
            }
        }
        if was_in_play != still_in_play {
            link.clear_relayed();
        }
    }

    fn send_readiness(
        &mut self,
        link: &LinkAccess<'_, Self>,
        send: &mut dyn FnMut(&Message, Channel) -> bool,
    ) {
        let (state, current) = {
            let shared = link.shared();
            (shared.state.clone(), shared.effective_local_readiness())
        };
        if state == NetState::WaitingForLauncher {
            return;
        }
        let now_ms = net_clock_ms();
        let due = match self.readiness_sent {
            None => true,
            Some((sent, _)) if sent != current => true,
            Some((_, at_ms)) => now_ms.saturating_sub(at_ms) >= HEARTBEAT_INTERVAL_MS,
        };
        if !due {
            return;
        }
        if send(&Message::Readiness(current), Channel::ReliableOrdered) {
            if self.readiness_sent.is_none_or(|(sent, _)| sent != current) {
                log_readiness(&format!("readiness: this game {current:?}"));
            }
            self.readiness_sent = Some((current, now_ms));
        }
    }
}

fn on_partner_state(link: &LinkAccess<'_, ReplicantNet>, state: PlayerState) {
    let mut shared = link.shared();
    let promoted = shared.partner.partner_sent_state();
    let name = shared.peer_name.clone().unwrap_or_default();
    let remote_slot = u8::from(shared.local_slot == Some(0));
    shared.remote = Some(RemotePlayer {
        name,
        slot: remote_slot,
        state,
        received_at: std::time::Instant::now(),
    });
    drop(shared);
    if promoted {
        log_readiness("readiness: partner in play (it sent its position)");
        post_partner_event(link, "'s game is in play");
    }
}

fn on_partner_notice(link: &LinkAccess<'_, ReplicantNet>, notice: PartnerNotice) {
    let Some(name) = link.shared().peer_name.clone() else {
        return;
    };
    match notice {
        PartnerNotice::WantsToChangeZone => {
            post_event(&format!("{name} wants to change zone"));
        }
        PartnerNotice::HostWantsToGo { phase } => {
            let area = phase
                .and_then(|phase| PHASE_TABLE.get(usize::from(phase)))
                .map_or("another area", |entry| entry.area);
            post_event(&format!("{name} wants to go to {area}"));
        }
        PartnerNotice::RejoiningYourArea => {
            post_event(&format!("{name} is rejoining your area"));
        }
        PartnerNotice::WantsToRideTheCart => {
            post_event(&partner_wants_to_ride_text(&name));
        }
    }
}

fn assume_silent_partner_in_play(link: &LinkAccess<'_, ReplicantNet>) {
    let assumed = link
        .shared()
        .partner
        .assume_in_play_if_silent(net_clock_ms());
    if assumed {
        log_readiness("readiness: partner never reported, assuming in play");
        post_partner_event(link, "'s game is in play");
    }
}

fn post_partner_event(link: &LinkAccess<'_, ReplicantNet>, suffix: &str) {
    let name = link.shared().peer_name.clone();
    if let Some(name) = name {
        post_event(&format!("{name}{suffix}"));
    }
}
