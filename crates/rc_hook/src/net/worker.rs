use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use rc_game::readiness::HEARTBEAT_INTERVAL_MS;
use rc_net::{Channel, Invite, LINK_INVITE_PREFIX, LINK_LAUNCHER_PEER, Transport, TransportEvent};
use rc_proto::{GameReadiness, Message, SessionOptions, decode, encode};

use crate::net::local_snapshot::LocalSnapshot;
use crate::net::readiness_log::log_readiness;
use crate::net::shared::{NetShared, NetState, RemotePlayer, SharedNet, net_clock_ms};
use crate::net::{MAX_RELAYED_INBOX, SEND_INTERVAL, WORKER_WAKE_INTERVAL};

pub(super) struct Worker<T: Transport> {
    transport: T,
    link_port: u16,
    shared: SharedNet,
    wants_snapshots: Arc<AtomicBool>,
    relay_inbox: Arc<Mutex<VecDeque<Message>>>,
    next_tick: u32,
    latest_snapshot: Option<LocalSnapshot>,
    last_sent: Option<Instant>,
    local_slot: Option<u8>,
    readiness_sent: Option<(GameReadiness, u64)>,
}

impl<T: Transport> Worker<T> {
    pub(super) fn new(
        transport: T,
        link_port: u16,
        shared: SharedNet,
        wants_snapshots: Arc<AtomicBool>,
        relay_inbox: Arc<Mutex<VecDeque<Message>>>,
    ) -> Self {
        Self {
            transport,
            link_port,
            shared,
            wants_snapshots,
            relay_inbox,
            next_tick: 0,
            latest_snapshot: None,
            last_sent: None,
            local_slot: None,
            readiness_sent: None,
        }
    }

    pub(super) fn run(mut self, snapshots: &Receiver<LocalSnapshot>, relays: &Receiver<Message>) {
        self.shared().link_port = Some(self.link_port);
        if let Ok(invite) = Invite::new(format!("{LINK_INVITE_PREFIX}{}", self.link_port)) {
            let _ = self.transport.join(&invite);
        }
        loop {
            match snapshots.recv_timeout(WORKER_WAKE_INTERVAL) {
                Ok(snapshot) => self.latest_snapshot = Some(snapshot),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            if let Some(snapshot) = snapshots.try_iter().last() {
                self.latest_snapshot = Some(snapshot);
            }
            self.pump(Instant::now());
            self.send_relayed(relays);
        }
    }

    fn pump(&mut self, now: Instant) {
        let mut events = Vec::new();
        self.transport.poll(&mut events);
        for event in events {
            match event {
                TransportEvent::PeerConnected(_) => self.on_launcher_linked(),
                TransportEvent::PeerDisconnected(..) => self.on_launcher_lost(),
                TransportEvent::Message { payload, .. } => self.on_payload(&payload),
            }
        }
        self.assume_silent_partner_in_play();
        self.send_readiness();
        self.send_latest_snapshot(now);
    }

    fn on_launcher_linked(&mut self) {
        self.clear_peer();
        self.readiness_sent = None;
        self.shared().state = NetState::LauncherLinked;
    }

    fn on_launcher_lost(&mut self) {
        self.clear_peer();
        self.shared().state = NetState::WaitingForLauncher;
    }

    fn clear_peer(&mut self) {
        self.wants_snapshots.store(false, Ordering::Relaxed);
        self.latest_snapshot = None;
        self.local_slot = None;
        let mut shared = self.shared();
        shared.peer_name = None;
        shared.rtt = None;
        shared.direct_path = None;
        shared.remote = None;
        shared.local = None;
        shared.local_slot = None;
        shared.options = SessionOptions::default();
        shared.partner.partner_gone();
        drop(shared);
        self.relay_inbox
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    fn on_payload(&mut self, payload: &[u8]) {
        let Ok(message) = decode(payload) else {
            return;
        };
        match message {
            Message::Welcome(welcome) => {
                self.local_slot = Some(welcome.slot);
                let mut shared = self.shared();
                shared.state = NetState::Connected;
                shared.peer_name = Some(welcome.host_name);
                shared.local_slot = Some(welcome.slot);
                shared.partner.partner_connected(net_clock_ms());
                drop(shared);
                self.wants_snapshots.store(true, Ordering::Relaxed);
            }
            Message::Readiness(readiness) => self.on_partner_readiness(readiness),
            Message::State(state) => {
                let mut shared = self.shared();
                let promoted = shared.partner.partner_sent_state();
                let name = shared.peer_name.clone().unwrap_or_default();
                let remote_slot = u8::from(self.local_slot == Some(0));
                shared.remote = Some(RemotePlayer {
                    name,
                    slot: remote_slot,
                    state,
                    received_at: Instant::now(),
                });
                drop(shared);
                if promoted {
                    log_readiness("readiness: partner in play (it sent its position)");
                }
            }
            Message::SessionOptions(options) => self.shared().options = options,
            Message::PathInfo(info) => {
                let mut shared = self.shared();
                shared.rtt = Some(Duration::from_micros(u64::from(info.rtt_micros)));
                shared.direct_path = Some(info.direct);
            }
            Message::Bye => {
                self.clear_peer();
                self.shared().state = NetState::LauncherLinked;
            }
            Message::Reject(reject) => {
                self.clear_peer();
                self.shared().state = NetState::Rejected(reject.reason);
            }
            relayed if relayed.is_relayed() => {
                if !self.shared().partner.is_in_play() {
                    return;
                }
                let mut inbox = self
                    .relay_inbox
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                if inbox.len() >= MAX_RELAYED_INBOX {
                    inbox.pop_front();
                }
                inbox.push_back(relayed);
            }
            _ => {}
        }
    }

    fn assume_silent_partner_in_play(&mut self) {
        let assumed = self
            .shared()
            .partner
            .assume_in_play_if_silent(net_clock_ms());
        if assumed {
            log_readiness("readiness: partner never reported, assuming in play");
        }
    }

    fn on_partner_readiness(&mut self, readiness: GameReadiness) {
        log_readiness(&format!("readiness: partner {readiness:?}"));
        let mut shared = self.shared();
        let was_in_play = shared.partner.is_in_play();
        shared.partner.partner_reported(readiness);
        let still_in_play = shared.partner.is_in_play();
        if was_in_play != still_in_play {
            shared.remote = None;
        }
        drop(shared);
        if was_in_play != still_in_play {
            self.relay_inbox
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clear();
        }
    }

    fn send_readiness(&mut self) {
        let (state, current) = {
            let shared = self.shared();
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
        let Ok(bytes) = encode(&Message::Readiness(current)) else {
            return;
        };
        if self
            .transport
            .send(LINK_LAUNCHER_PEER, Channel::ReliableOrdered, &bytes)
            .is_ok()
        {
            if self.readiness_sent.is_none_or(|(sent, _)| sent != current) {
                log_readiness(&format!("readiness: this game {current:?}"));
            }
            self.readiness_sent = Some((current, now_ms));
        }
    }

    fn send_latest_snapshot(&mut self, now: Instant) {
        let due = self
            .last_sent
            .is_none_or(|sent| now.saturating_duration_since(sent) >= SEND_INTERVAL);
        if !due || !self.wants_snapshots.load(Ordering::Relaxed) {
            return;
        }
        let Some(snapshot) = self.latest_snapshot.take() else {
            return;
        };
        self.next_tick = self.next_tick.wrapping_add(1);
        let message = Message::State(snapshot.to_player_state(self.next_tick));
        let Ok(bytes) = encode(&message) else {
            return;
        };
        if self
            .transport
            .send(LINK_LAUNCHER_PEER, Channel::UnreliableLatest, &bytes)
            .is_ok()
        {
            self.last_sent = Some(now);
        }
    }

    fn send_relayed(&mut self, relays: &Receiver<Message>) {
        let connected =
            self.wants_snapshots.load(Ordering::Relaxed) && self.shared().partner.is_in_play();
        for message in relays.try_iter() {
            if !connected {
                continue;
            }
            let Ok(bytes) = encode(&message) else {
                continue;
            };
            let channel = if message.is_latest_wins() {
                Channel::UnreliableLatest
            } else {
                Channel::ReliableOrdered
            };
            let _ = self.transport.send(LINK_LAUNCHER_PEER, channel, &bytes);
        }
    }

    fn shared(&self) -> MutexGuard<'_, NetShared> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
