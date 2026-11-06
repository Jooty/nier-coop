use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use rc_net::{Channel, Invite, LINK_INVITE_PREFIX, LINK_LAUNCHER_PEER, Transport, TransportEvent};
use rc_proto::{Message, decode, encode};

use crate::net::local_snapshot::LocalSnapshot;
use crate::net::shared::{NetShared, NetState, RemotePlayer, SharedNet};
use crate::net::{SEND_INTERVAL, WORKER_WAKE_INTERVAL};

pub(super) struct Worker<T: Transport> {
    transport: T,
    link_port: u16,
    shared: SharedNet,
    wants_snapshots: Arc<AtomicBool>,
    next_tick: u32,
    latest_snapshot: Option<LocalSnapshot>,
    last_sent: Option<Instant>,
    local_slot: Option<u8>,
}

impl<T: Transport> Worker<T> {
    pub(super) fn new(
        transport: T,
        link_port: u16,
        shared: SharedNet,
        wants_snapshots: Arc<AtomicBool>,
    ) -> Self {
        Self {
            transport,
            link_port,
            shared,
            wants_snapshots,
            next_tick: 0,
            latest_snapshot: None,
            last_sent: None,
            local_slot: None,
        }
    }

    pub(super) fn run(mut self, snapshots: &Receiver<LocalSnapshot>) {
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
        self.send_latest_snapshot(now);
    }

    fn on_launcher_linked(&mut self) {
        self.clear_peer();
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
                drop(shared);
                self.wants_snapshots.store(true, Ordering::Relaxed);
            }
            Message::State(state) => {
                let mut shared = self.shared();
                let name = shared.peer_name.clone().unwrap_or_default();
                let remote_slot = u8::from(self.local_slot == Some(0));
                shared.remote = Some(RemotePlayer {
                    name,
                    slot: remote_slot,
                    state,
                    received_at: Instant::now(),
                });
            }
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
            _ => {}
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

    fn shared(&self) -> MutexGuard<'_, NetShared> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
