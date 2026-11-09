use std::collections::HashMap;
use std::time::{Duration, Instant};

use rc_proto::{
    Hello, MAX_PLAYERS, Message, PROTOCOL_VERSION, Ping, PlayerState, Pong, Reject, RejectReason,
    Welcome, WireError, decode, encode,
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

pub struct Session<T: Transport> {
    transport: T,
    config: SessionConfig,
    peers: HashMap<PeerId, PeerSession>,
    next_nonce: u32,
    local_slot: Option<u8>,
}

impl<T: Transport> Session<T> {
    pub fn new(transport: T, config: SessionConfig) -> Self {
        let local_slot = (config.role == Role::Host).then_some(0);
        Self {
            transport,
            config,
            peers: HashMap::new(),
            next_nonce: 0,
            local_slot,
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn local_slot(&self) -> Option<u8> {
        self.local_slot
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

    pub fn send_state(&mut self, state: &PlayerState) -> Result<(), WireError> {
        let message = Message::State(*state);
        message.validate()?;
        for peer in self.ready_peers() {
            self.send_to(peer, &message);
        }
        Ok(())
    }

    pub fn send_relayed(&mut self, message: &Message) -> Result<(), WireError> {
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
            self.send_to(peer, &Message::Bye);
        }
    }

    pub fn poll(&mut self, now: Instant, out: &mut Vec<SessionEvent>) {
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
            let hello = Message::Hello(Hello {
                version: PROTOCOL_VERSION,
                build: self.config.build,
                name: self.config.name.clone(),
            });
            self.send_to(peer, &hello);
        }
    }

    fn on_disconnected(
        &mut self,
        peer: PeerId,
        reason: DisconnectReason,
        out: &mut Vec<SessionEvent>,
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
        out: &mut Vec<SessionEvent>,
    ) {
        let Some(session) = self.peers.get_mut(&peer) else {
            return;
        };
        let phase = session.phase;
        if matches!(phase, Phase::Refused { .. }) {
            return;
        }
        session.last_heard = now;
        let Ok(message) = decode(payload) else {
            self.drop_peer(peer, LeaveReason::ProtocolViolation, out);
            return;
        };
        match (phase, self.config.role, message) {
            (Phase::Handshaking, Role::Host, Message::Hello(hello)) => {
                self.on_hello(peer, &hello, now, out);
            }
            (Phase::Handshaking, Role::Guest, Message::Welcome(welcome)) => {
                self.on_welcome(peer, &welcome, out);
            }
            (Phase::Handshaking, Role::Guest, Message::Reject(reject)) => {
                self.peers.remove(&peer);
                self.transport.disconnect(peer);
                out.push(SessionEvent::Rejected {
                    reason: reject.reason,
                });
            }
            (Phase::Ready, _, message) => self.on_ready_message(peer, message, now, out),
            _ => self.drop_peer(peer, LeaveReason::ProtocolViolation, out),
        }
    }

    fn on_hello(&mut self, peer: PeerId, hello: &Hello, now: Instant, out: &mut Vec<SessionEvent>) {
        let refusal = if hello.version != PROTOCOL_VERSION {
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
        out: &mut Vec<SessionEvent>,
    ) {
        self.send_to(peer, &Message::Reject(Reject { reason }));
        if let Some(session) = self.peers.get_mut(&peer) {
            session.phase = Phase::Refused {
                drop_at: now + REFUSAL_LINGER,
            };
        }
        out.push(SessionEvent::PeerRefused { peer, reason });
    }

    fn admit(&mut self, peer: PeerId, name: &str, out: &mut Vec<SessionEvent>) {
        let Some(slot) = self.free_slot() else {
            return;
        };
        if let Some(session) = self.peers.get_mut(&peer) {
            session.phase = Phase::Ready;
            session.slot = slot;
        }
        let welcome = Message::Welcome(Welcome {
            version: PROTOCOL_VERSION,
            slot,
            host_name: self.config.name.clone(),
        });
        self.send_to(peer, &welcome);
        out.push(SessionEvent::PeerJoined {
            peer,
            name: name.to_owned(),
            slot,
        });
    }

    fn on_welcome(&mut self, peer: PeerId, welcome: &Welcome, out: &mut Vec<SessionEvent>) {
        if welcome.version != PROTOCOL_VERSION {
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
        message: Message,
        now: Instant,
        out: &mut Vec<SessionEvent>,
    ) {
        match message {
            Message::State(state) => {
                let fresh = self
                    .peers
                    .get_mut(&peer)
                    .is_some_and(|session| session.accepts_tick(state.tick));
                if fresh {
                    out.push(SessionEvent::PeerState { peer, state });
                }
            }
            relayed @ (Message::Combat(_)
            | Message::FollowPhase(_)
            | Message::EnemySnapshot(_)
            | Message::EnemySpawned(_)
            | Message::EnemyCreation(_)
            | Message::EnemyGone(_)
            | Message::SessionOptions(_)
            | Message::Readiness(_)
            | Message::Notice(_)
            | Message::Effect(_)
            | Message::Sound(_)
            | Message::EffectStop(_)
            | Message::Music(_)
            | Message::Pickup(_)
            | Message::Sky(_)
            | Message::SkyRecord(_)
            | Message::SkyAssets(_)
            | Message::Dialogue(_)
            | Message::Fade(_)
            | Message::NpcSnapshot(_)
            | Message::NpcTalk(_)
            | Message::EnemyHit(_)
            | Message::PlayerHit(_)
            | Message::EnemyKilled(_)
            | Message::EnemyDrop(_)
            | Message::DoorChange(_)
            | Message::DoorRequest(_)
            | Message::DoorReport(_)
            | Message::CutsceneSkip(_)
            | Message::StoryRequest(_)
            | Message::StoryPart(_)
            | Message::StoryChanges(_)
            | Message::StoryDigest(_)
            | Message::EnemyWordDrop(_)
            | Message::ContainerOpened(_)
            | Message::StoryWrite(_)
            | Message::TalkChoice(_)
            | Message::StoryTrigger(_)
            | Message::StoryRun(_)
            | Message::ScriptRoll(_)
            | Message::StoryRefused(_)
            | Message::ScriptMessage(_)
            | Message::EndingRecords(_)
            | Message::LobbyReady(_)
            | Message::LobbyStart(_)
            | Message::PartnerBody(_)
            | Message::TutorialPrompt(_)
            | Message::BookEffect(_)
            | Message::BookSound(_)
            | Message::BulletEffect(_)
            | Message::PrologueLevel(_)
            | Message::PowerModel(_)
            | Message::BulletFlight(_)
            | Message::SceneFollow(_)
            | Message::CallbackFired(_)
            | Message::Desync(_)
            | Message::EnemyBulletFlight(_)
            | Message::PausedHits(_)
            | Message::EnemyBodyGone(_)
            | Message::PowerMotion(_)
            | Message::PowerEnded(_)
            | Message::WorldEffect(_)
            | Message::WorldEffectMove(_)
            | Message::VoiceCharacter(_)
            | Message::StoryJump(_)
            | Message::LobbyBody(_)
            | Message::ItemCounts(_)
            | Message::ItemTake(_)
            | Message::TeleportHere(_)
            | Message::PartnerBook(_)
            | Message::BlockMoved(_)
            | Message::DummyHit(_)
            | Message::WatchedCallbackFired(_)
            | Message::SharedFunctionRan(_)
            | Message::MessageHandlerRan(_)
            | Message::FightStream(_)
            | Message::FightWindow(_)) => out.push(SessionEvent::PeerRelayed {
                peer,
                message: relayed,
            }),
            Message::Ping(Ping { nonce }) => self.send_to(peer, &Message::Pong(Pong { nonce })),
            Message::Pong(Pong { nonce }) => {
                if let Some(session) = self.peers.get_mut(&peer)
                    && let Some((expected, sent_at)) = session.outstanding_ping
                    && expected == nonce
                {
                    session.rtt = Some(now.saturating_duration_since(sent_at));
                    session.pongs_received += 1;
                    session.outstanding_ping = None;
                }
            }
            Message::Bye => self.drop_peer(peer, LeaveReason::Quit, out),
            Message::Hello(_)
            | Message::Welcome(_)
            | Message::Reject(_)
            | Message::PathInfo(_)
            | Message::QuitGame
            | Message::Settings(_) => {
                self.drop_peer(peer, LeaveReason::ProtocolViolation, out);
            }
        }
    }

    fn run_timers(&mut self, now: Instant, out: &mut Vec<SessionEvent>) {
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
        self.send_to(peer, &Message::Ping(Ping { nonce }));
    }

    fn free_slot(&self) -> Option<u8> {
        (1..MAX_PLAYERS).find(|slot| {
            !self
                .peers
                .values()
                .any(|session| session.phase == Phase::Ready && session.slot == *slot)
        })
    }

    fn drop_peer(&mut self, peer: PeerId, reason: LeaveReason, out: &mut Vec<SessionEvent>) {
        self.forget_peer(peer, reason, out);
        self.transport.disconnect(peer);
    }

    fn forget_peer(&mut self, peer: PeerId, reason: LeaveReason, out: &mut Vec<SessionEvent>) {
        if let Some(session) = self.peers.remove(&peer)
            && session.phase == Phase::Ready
        {
            out.push(SessionEvent::PeerLeft { peer, reason });
        }
    }

    fn send_to(&mut self, peer: PeerId, message: &Message) {
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
