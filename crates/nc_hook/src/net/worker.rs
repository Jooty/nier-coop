use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, MutexGuard, PoisonError};
use std::time::Instant;

use nc_net::link_traffic::{LinkTraffic, RefusalLog};
use nc_net::{Channel, PeerId, Transport, TransportEvent};
use nc_wire::{GameMessage, LinkPayload, WireError, decode, encode};

use crate::diagnostics::log_line;
use crate::net::link_peer::LinkPeer;
use crate::net::net_game::{LinkAccess, NetGame, NetSharedOf, SharedNetOf};
use crate::net::net_state::NetState;
use crate::net::{SEND_INTERVAL, WORKER_WAKE_INTERVAL};
use crate::relay::RelayInbox;

pub(super) struct Worker<G: NetGame, T: Transport> {
    game: G,
    transport: T,
    link: LinkPeer,
    shared: SharedNetOf<G>,
    wants_snapshots: Arc<AtomicBool>,
    relay_inbox: Arc<RelayInbox<G::Message>>,
    next_tick: u32,
    latest_snapshot: Option<G::Snapshot>,
    last_sent: Option<Instant>,
    traffic: LinkTraffic,
    refusals: RefusalLog,
}

impl<G: NetGame, T: Transport> Worker<G, T> {
    pub(super) fn new(
        game: G,
        transport: T,
        link: LinkPeer,
        shared: SharedNetOf<G>,
        wants_snapshots: Arc<AtomicBool>,
        relay_inbox: Arc<RelayInbox<G::Message>>,
    ) -> Self {
        Self {
            game,
            transport,
            link,
            shared,
            wants_snapshots,
            relay_inbox,
            next_tick: 0,
            latest_snapshot: None,
            last_sent: None,
            traffic: LinkTraffic::default(),
            refusals: RefusalLog::default(),
        }
    }

    pub(super) fn run(
        mut self,
        snapshots: &Receiver<G::Snapshot>,
        relays: &Receiver<G::Message>,
        launcher_messages: &Receiver<G::Message>,
    ) {
        lock_shared::<G>(&self.shared).link_port = Some(self.link.port);
        if let Some(invite) = self.link.invite.clone() {
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
            self.send_launcher_messages(launcher_messages);
        }
    }

    fn pump(&mut self, now: Instant) {
        let mut events = Vec::new();
        self.transport.poll(&mut events);
        for event in events {
            match event {
                TransportEvent::PeerConnected(_) => self.on_launcher_linked(),
                TransportEvent::PeerDisconnected(_, reason) => {
                    log_line(&format!(
                        "net: the launcher link {}; {}",
                        reason.label(),
                        self.traffic.describe(now)
                    ));
                    self.on_launcher_lost();
                }
                TransportEvent::Message { payload, .. } => self.on_payload(&payload),
            }
        }
        self.run_game_idle();
        self.send_latest_snapshot(now);
    }

    fn run_game_idle(&mut self) {
        let access = LinkAccess::<G>::new(&self.shared, &self.relay_inbox);
        let transport = &mut self.transport;
        let peer = self.link.peer;
        self.game.on_idle(&access, &mut |message, channel| {
            send_encoded(transport, peer, channel, message)
        });
    }

    fn on_launcher_linked(&mut self) {
        self.clear_peer();
        self.game.on_launcher_linked();
        lock_shared::<G>(&self.shared).state = NetState::LauncherLinked;
    }

    fn on_launcher_lost(&mut self) {
        self.clear_peer();
        lock_shared::<G>(&self.shared).state = NetState::WaitingForLauncher;
    }

    fn clear_peer(&mut self) {
        self.wants_snapshots.store(false, Ordering::Relaxed);
        self.latest_snapshot = None;
        let departed = {
            let mut shared = lock_shared::<G>(&self.shared);
            let departed = shared.peer_name.take();
            shared.rtt = None;
            shared.direct_path = None;
            shared.remote = None;
            shared.local = None;
            shared.local_slot = None;
            self.game.on_peer_cleared(&mut shared.extras);
            departed
        };
        if let Some(name) = departed {
            self.game.announce(&format!("{name} left"));
        }
        self.relay_inbox.clear();
    }

    fn on_payload(&mut self, payload: &[u8]) {
        let message = match decode::<G::Message>(payload) {
            Ok(message) => message,
            Err(error) => {
                self.log_refused("undecodable message", &error);
                return;
            }
        };
        self.traffic
            .note_received(G::kind_name(&message), Instant::now());
        match message.into_link() {
            Ok(LinkPayload::Welcome(welcome)) => {
                {
                    let mut shared = lock_shared::<G>(&self.shared);
                    shared.state = NetState::Connected;
                    shared.peer_name = Some(welcome.host_name.clone());
                    shared.local_slot = Some(welcome.slot);
                    self.game
                        .on_connected(&mut shared.extras, &welcome.host_name);
                }
                self.game.announce(&format!("{} joined", welcome.host_name));
                self.wants_snapshots.store(true, Ordering::Relaxed);
            }
            Ok(LinkPayload::Bye) => {
                log_line(&format!(
                    "net: the launcher says the other player left; {}",
                    self.traffic.describe(Instant::now())
                ));
                self.clear_peer();
                lock_shared::<G>(&self.shared).state = NetState::LauncherLinked;
            }
            Ok(LinkPayload::Reject(reject)) => {
                self.clear_peer();
                lock_shared::<G>(&self.shared).state = NetState::Rejected(reject.reason);
            }
            Ok(_) => {}
            Err(message) => {
                let access = LinkAccess::<G>::new(&self.shared, &self.relay_inbox);
                self.game.on_message(&access, message);
            }
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
        let message = self.game.snapshot_message(&snapshot, self.next_tick);
        let Ok(bytes) = encode(&message) else {
            return;
        };
        if self
            .transport
            .send(self.link.peer, Channel::UnreliableLatest, &bytes)
            .is_ok()
        {
            self.last_sent = Some(now);
            self.traffic.note_sent(G::kind_name(&message), now);
        }
    }

    fn log_refused(&mut self, kind: &'static str, error: &WireError) {
        if self.refusals.should_log(kind, Instant::now()) {
            log_line(&format!(
                "net: refused to send {kind}, it was dropped: {error}"
            ));
        }
    }

    fn send_relayed(&mut self, relays: &Receiver<G::Message>) {
        let connected = self.wants_snapshots.load(Ordering::Relaxed)
            && G::peer_in_play(&lock_shared::<G>(&self.shared).extras);
        for message in relays.try_iter() {
            if !connected {
                continue;
            }
            let bytes = match encode(&message) {
                Ok(bytes) => bytes,
                Err(error) => {
                    self.log_refused(G::kind_name(&message), &error);
                    continue;
                }
            };
            let channel = if message.is_latest_wins() {
                Channel::UnreliableLatest
            } else {
                Channel::ReliableOrdered
            };
            if self.transport.send(self.link.peer, channel, &bytes).is_ok() {
                self.traffic
                    .note_sent(G::kind_name(&message), Instant::now());
            }
        }
    }

    fn send_launcher_messages(&mut self, messages: &Receiver<G::Message>) {
        let linked = lock_shared::<G>(&self.shared).state != NetState::WaitingForLauncher;
        for message in messages.try_iter() {
            if !linked {
                continue;
            }
            let bytes = match encode(&message) {
                Ok(bytes) => bytes,
                Err(error) => {
                    self.log_refused(G::kind_name(&message), &error);
                    continue;
                }
            };
            let _ = self
                .transport
                .send(self.link.peer, Channel::ReliableOrdered, &bytes);
        }
    }
}

fn lock_shared<G: NetGame>(shared: &SharedNetOf<G>) -> MutexGuard<'_, NetSharedOf<G>> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

fn send_encoded<M: GameMessage>(
    transport: &mut impl Transport,
    peer: PeerId,
    channel: Channel,
    message: &M,
) -> bool {
    let Ok(bytes) = encode(message) else {
        return false;
    };
    transport.send(peer, channel, &bytes).is_ok()
}
