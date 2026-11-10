use std::collections::HashMap;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use nc_wire::{
    GameMessage, Hello, LinkPayload, Ping, Pong, Reject, RejectReason, Welcome, WireError, decode,
    encode,
};

use crate::session::peer::{PeerSession, Phase};
use crate::session::{
    HANDSHAKE_TIMEOUT, KEEPALIVE_INTERVAL, LeaveReason, PEER_TIMEOUT, REFUSAL_LINGER, Role,
    SessionConfig, SessionEvent,
};
use crate::transport::{
    Channel, DisconnectReason, Invite, NetError, PeerId, Transport, TransportEvent,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PingCounts {
    pub sent: u32,
    pub answered: u32,
    pub awaiting_answer: bool,
}

pub struct Session<T: Transport, M: GameMessage> {
    transport: T,
    config: SessionConfig,
    peers: HashMap<PeerId, PeerSession>,
    next_nonce: u32,
    local_slot: Option<u8>,
    refused_peer_version: Option<u16>,
    message: PhantomData<fn() -> M>,
}

impl<T: Transport, M: GameMessage> Session<T, M> {
    pub fn new(transport: T, config: SessionConfig) -> Self {
        let local_slot = (config.role == Role::Host).then_some(0);
        Self {
            transport,
            config,
            peers: HashMap::new(),
            next_nonce: 0,
            local_slot,
            refused_peer_version: None,
            message: PhantomData,
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn local_slot(&self) -> Option<u8> {
        self.local_slot
    }

    /// The protocol version the last player refused for a version mismatch asked with.
    pub fn refused_peer_version(&self) -> Option<u16> {
        self.refused_peer_version
    }

    pub fn host(&mut self) -> Result<Invite, NetError> {
        self.transport.host()
    }

    pub fn join(&mut self, invite: &Invite) -> Result<(), NetError> {
        self.transport.join(invite)
    }

    pub fn rtt(&self, peer: PeerId) -> Option<Duration> {
        self.peers.get(&peer)?.rtt
    }

    pub fn ping_counts(&self, peer: PeerId) -> Option<PingCounts> {
        let session = self.peers.get(&peer)?;
        Some(PingCounts {
            sent: session.pings_sent,
            answered: session.pongs_received,
            awaiting_answer: session.outstanding_ping.is_some(),
        })
    }

    pub fn ready_peers(&self) -> Vec<PeerId> {
        self.peers
            .iter()
            .filter(|(_, session)| session.phase == Phase::Ready)
            .map(|(peer, _)| *peer)
            .collect()
    }

    pub fn send_ordered(&mut self, message: &M) -> Result<(), WireError> {
        if message.ordered_tick().is_none() {
            return Err(WireError::OutOfRange { field: "ordered" });
        }
        message.validate()?;
        for peer in self.ready_peers() {
            self.send_to(peer, message);
        }
        Ok(())
    }

    pub fn send_relayed(&mut self, message: &M) -> Result<(), WireError> {
        if !message.is_relayed() {
            return Err(WireError::OutOfRange { field: "relayed" });
        }
        message.validate()?;
        for peer in self.ready_peers() {
            self.send_to(peer, message);
        }
        Ok(())
    }

    pub fn close(&mut self) {
        for peer in self.ready_peers() {
            self.send_link(peer, LinkPayload::Bye);
        }
    }

    pub fn poll(&mut self, now: Instant, out: &mut Vec<SessionEvent<M>>) {
        let mut incoming = Vec::new();
        self.transport.poll(&mut incoming);
        for event in incoming {
            match event {
                TransportEvent::PeerConnected(peer) => self.on_connected(peer, now),
                TransportEvent::PeerDisconnected(peer, reason) => {
                    self.on_disconnected(peer, reason, out);
                }
                TransportEvent::Message { from, payload, .. } => {
                    self.on_payload(from, &payload, now, out);
                }
            }
        }
        self.run_timers(now, out);
    }

    fn on_connected(&mut self, peer: PeerId, now: Instant) {
        self.peers.insert(peer, PeerSession::new(now));
        if self.config.role == Role::Guest {
            let hello = LinkPayload::Hello(Hello {
                version: M::PROTOCOL_VERSION,
                build: self.config.build,
                name: self.config.name.clone(),
            });
            self.send_link(peer, hello);
        }
    }

    fn on_disconnected(
        &mut self,
        peer: PeerId,
        reason: DisconnectReason,
        out: &mut Vec<SessionEvent<M>>,
    ) {
        let leave = match reason {
            DisconnectReason::TimedOut => LeaveReason::TimedOut,
            DisconnectReason::Closed | DisconnectReason::Failed => LeaveReason::TransportClosed,
        };
        self.forget_peer(peer, leave, out);
    }

    fn on_payload(
        &mut self,
        peer: PeerId,
        payload: &[u8],
        now: Instant,
        out: &mut Vec<SessionEvent<M>>,
    ) {
        let Some(session) = self.peers.get_mut(&peer) else {
            return;
        };
        let phase = session.phase;
        if matches!(phase, Phase::Refused { .. }) {
            return;
        }
        session.last_heard = now;
        let Ok(message) = decode::<M>(payload) else {
            self.drop_peer(peer, LeaveReason::ProtocolViolation, out);
            return;
        };
        if phase == Phase::Ready {
            self.on_ready_message(peer, message, now, out);
            return;
        }
        match (phase, self.config.role, message.into_link()) {
            (Phase::Handshaking, Role::Host, Ok(LinkPayload::Hello(hello))) => {
                self.on_hello(peer, &hello, now, out);
            }
            (Phase::Handshaking, Role::Guest, Ok(LinkPayload::Welcome(welcome))) => {
                self.on_welcome(peer, &welcome, out);
            }
            (Phase::Handshaking, Role::Guest, Ok(LinkPayload::Reject(reject))) => {
                self.peers.remove(&peer);
                self.transport.disconnect(peer);
                out.push(SessionEvent::Rejected {
                    reason: reject.reason,
                });
            }
            _ => self.drop_peer(peer, LeaveReason::ProtocolViolation, out),
        }
    }

    fn on_hello(
        &mut self,
        peer: PeerId,
        hello: &Hello,
        now: Instant,
        out: &mut Vec<SessionEvent<M>>,
    ) {
        let refusal = if hello.version != M::PROTOCOL_VERSION {
            self.refused_peer_version = Some(hello.version);
            Some(RejectReason::VersionMismatch)
        } else if hello.build != self.config.build {
            Some(RejectReason::BuildMismatch)
        } else if self.free_slot().is_none() {
            Some(RejectReason::SessionFull)
        } else {
            None
        };
        if let Some(reason) = refusal {
            self.refuse(peer, reason, now, out);
            return;
        }
        self.admit(peer, &hello.name, out);
    }

    fn refuse(
        &mut self,
        peer: PeerId,
        reason: RejectReason,
        now: Instant,
        out: &mut Vec<SessionEvent<M>>,
    ) {
        self.send_link(peer, LinkPayload::Reject(Reject { reason }));
        if let Some(session) = self.peers.get_mut(&peer) {
            session.phase = Phase::Refused {
                drop_at: now + REFUSAL_LINGER,
            };
        }
        out.push(SessionEvent::PeerRefused { peer, reason });
    }

    fn admit(&mut self, peer: PeerId, name: &str, out: &mut Vec<SessionEvent<M>>) {
        let Some(slot) = self.free_slot() else {
            return;
        };
        if let Some(session) = self.peers.get_mut(&peer) {
            session.phase = Phase::Ready;
            session.slot = slot;
        }
        let welcome = LinkPayload::Welcome(Welcome {
            version: M::PROTOCOL_VERSION,
            slot,
            host_name: self.config.name.clone(),
        });
        self.send_link(peer, welcome);
        out.push(SessionEvent::PeerJoined {
            peer,
            name: name.to_owned(),
            slot,
        });
    }

    fn on_welcome(&mut self, peer: PeerId, welcome: &Welcome, out: &mut Vec<SessionEvent<M>>) {
        if welcome.version != M::PROTOCOL_VERSION {
            self.peers.remove(&peer);
            self.transport.disconnect(peer);
            out.push(SessionEvent::Rejected {
                reason: RejectReason::VersionMismatch,
            });
            return;
        }
        if let Some(session) = self.peers.get_mut(&peer) {
            session.phase = Phase::Ready;
            session.slot = 0;
        }
        self.local_slot = Some(welcome.slot);
        out.push(SessionEvent::PeerJoined {
            peer,
            name: welcome.host_name.clone(),
            slot: 0,
        });
    }

    fn on_ready_message(
        &mut self,
        peer: PeerId,
        message: M,
        now: Instant,
        out: &mut Vec<SessionEvent<M>>,
    ) {
        let message = match message.into_link() {
            Ok(link) => {
                self.on_ready_link(peer, link, now, out);
                return;
            }
            Err(message) => message,
        };
        if let Some(tick) = message.ordered_tick() {
            let fresh = self
                .peers
                .get_mut(&peer)
                .is_some_and(|session| session.accepts_tick(tick));
            if fresh {
                out.push(SessionEvent::PeerOrdered { peer, message });
            }
        } else if message.is_relayed() {
            out.push(SessionEvent::PeerRelayed { peer, message });
        } else {
            self.drop_peer(peer, LeaveReason::ProtocolViolation, out);
        }
    }

    fn on_ready_link(
        &mut self,
        peer: PeerId,
        link: LinkPayload,
        now: Instant,
        out: &mut Vec<SessionEvent<M>>,
    ) {
        match link {
            LinkPayload::Ping(Ping { nonce }) => {
                self.send_link(peer, LinkPayload::Pong(Pong { nonce }));
            }
            LinkPayload::Pong(Pong { nonce }) => {
                if let Some(session) = self.peers.get_mut(&peer)
                    && let Some((expected, sent_at)) = session.outstanding_ping
                    && expected == nonce
                {
                    session.rtt = Some(now.saturating_duration_since(sent_at));
                    session.pongs_received += 1;
                    session.outstanding_ping = None;
                }
            }
            LinkPayload::Bye => self.drop_peer(peer, LeaveReason::Quit, out),
            LinkPayload::Hello(_) | LinkPayload::Welcome(_) | LinkPayload::Reject(_) => {
                self.drop_peer(peer, LeaveReason::ProtocolViolation, out);
            }
        }
    }

    fn run_timers(&mut self, now: Instant, out: &mut Vec<SessionEvent<M>>) {
        let peers: Vec<PeerId> = self.peers.keys().copied().collect();
        for peer in peers {
            let Some(session) = self.peers.get(&peer) else {
                continue;
            };
            let phase = session.phase;
            let silent_for = now.saturating_duration_since(session.last_heard);
            let handshaking_for = now.saturating_duration_since(session.connected_at);
            let since_ping = now.saturating_duration_since(session.last_ping_sent);
            match phase {
                Phase::Refused { drop_at } if now >= drop_at => {
                    self.drop_peer(peer, LeaveReason::ProtocolViolation, out);
                }
                Phase::Refused { .. } => {}
                Phase::Handshaking if handshaking_for > HANDSHAKE_TIMEOUT => {
                    self.drop_peer(peer, LeaveReason::TimedOut, out);
                }
                Phase::Handshaking => {}
                Phase::Ready if silent_for > PEER_TIMEOUT => {
                    self.drop_peer(peer, LeaveReason::TimedOut, out);
                }
                Phase::Ready if since_ping >= KEEPALIVE_INTERVAL => self.ping(peer, now),
                Phase::Ready => {}
            }
        }
    }

    fn ping(&mut self, peer: PeerId, now: Instant) {
        self.next_nonce = self.next_nonce.wrapping_add(1);
        let nonce = self.next_nonce;
        if let Some(session) = self.peers.get_mut(&peer) {
            session.last_ping_sent = now;
            session.outstanding_ping = Some((nonce, now));
            session.pings_sent += 1;
        }
        self.send_link(peer, LinkPayload::Ping(Ping { nonce }));
    }

    fn free_slot(&self) -> Option<u8> {
        (1..M::MAX_PLAYERS).find(|slot| {
            !self
                .peers
                .values()
                .any(|session| session.phase == Phase::Ready && session.slot == *slot)
        })
    }

    fn drop_peer(&mut self, peer: PeerId, reason: LeaveReason, out: &mut Vec<SessionEvent<M>>) {
        self.forget_peer(peer, reason, out);
        self.transport.disconnect(peer);
    }

    fn forget_peer(&mut self, peer: PeerId, reason: LeaveReason, out: &mut Vec<SessionEvent<M>>) {
        if let Some(session) = self.peers.remove(&peer)
            && session.phase == Phase::Ready
        {
            out.push(SessionEvent::PeerLeft { peer, reason });
        }
    }

    fn send_link(&mut self, peer: PeerId, link: LinkPayload) {
        self.send_to(peer, &M::from_link(link));
    }

    fn send_to(&mut self, peer: PeerId, message: &M) {
        let Ok(bytes) = encode(message) else {
            return;
        };
        let channel = if message.is_latest_wins() {
            Channel::UnreliableLatest
        } else {
            Channel::ReliableOrdered
        };
        let _ = self.transport.send(peer, channel, &bytes);
    }
}
