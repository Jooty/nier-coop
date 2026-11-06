use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use rc_net::{
    Channel, Invite, LINK_GAME_PEER, LocalLinkTransport, PeerId, Role, Session, SessionConfig,
    SessionEvent, Transport, TransportEvent,
};
use rc_proto::{Message, PROTOCOL_VERSION, PathInfo, RejectReason, Welcome, decode, encode};

use crate::{
    ConnectionState, INVALID_TICKET_TEXT, LauncherConfig, LauncherEvent, LinkStatus, NetError,
    PATH_INFO_INTERVAL, TransportFactory,
};

type RemoteSession = Session<Box<dyn Transport>>;

pub(crate) struct Bridge {
    link: LocalLinkTransport,
    factory: TransportFactory,
    config: LauncherConfig,
    status: Arc<Mutex<LinkStatus>>,
    events: Sender<LauncherEvent>,
    remote: Option<RemoteSession>,
    remote_peer: Option<(PeerId, String)>,
    last_path_info: Option<Instant>,
    last_reject: Option<RejectReason>,
}

impl Bridge {
    pub(crate) fn new(
        link: LocalLinkTransport,
        factory: TransportFactory,
        config: LauncherConfig,
        status: Arc<Mutex<LinkStatus>>,
        events: Sender<LauncherEvent>,
    ) -> Self {
        Self {
            link,
            factory,
            config,
            status,
            events,
            remote: None,
            remote_peer: None,
            last_path_info: None,
            last_reject: None,
        }
    }

    pub(crate) fn host(&mut self, password: Option<String>) -> Result<String, NetError> {
        self.stop();
        let mut session = self.new_session(Role::Host, password)?;
        let invite = session
            .host()
            .map_err(|error| NetError::Failed(error.to_string()))?;
        let ticket = invite.as_str().to_owned();
        self.remote = Some(session);
        let mut status = self.status();
        status.state = ConnectionState::Hosting;
        status.ticket = Some(ticket.clone());
        drop(status);
        self.emit(LauncherEvent::Hosting {
            ticket: ticket.clone(),
        });
        Ok(ticket)
    }

    pub(crate) fn join(&mut self, ticket: &str, password: Option<String>) -> Result<(), NetError> {
        let invite = Invite::new(ticket.trim().to_owned())
            .map_err(|_| NetError::Failed(INVALID_TICKET_TEXT.to_owned()))?;
        self.stop();
        let mut session = self.new_session(Role::Guest, password)?;
        session
            .join(&invite)
            .map_err(|_| NetError::Failed(INVALID_TICKET_TEXT.to_owned()))?;
        self.remote = Some(session);
        self.status().state = ConnectionState::Joining;
        self.emit(LauncherEvent::Joining);
        Ok(())
    }

    pub(crate) fn stop(&mut self) {
        self.last_reject = None;
        let had_peer = self.remote_peer.is_some();
        if let Some(mut session) = self.remote.take() {
            session.close();
        }
        self.forget_peer();
        if had_peer {
            self.send_to_game(&Message::Bye);
        }
        let mut status = self.status();
        status.state = ConnectionState::Idle;
        status.ticket = None;
    }

    pub(crate) fn pump(&mut self, now: Instant) {
        self.pump_link();
        self.pump_remote(now);
        self.send_path_info(now);
    }

    fn new_session(&self, role: Role, password: Option<String>) -> Result<RemoteSession, NetError> {
        let transport = (self.factory)().map_err(|error| NetError::Failed(error.to_string()))?;
        let mut session = Session::new(
            transport,
            SessionConfig {
                role,
                name: self.config.name.clone(),
                build: self.config.build,
            },
        );
        session.set_password(password);
        Ok(session)
    }

    fn pump_link(&mut self) {
        let mut incoming = Vec::new();
        self.link.poll(&mut incoming);
        for event in incoming {
            match event {
                TransportEvent::PeerConnected(_) => {
                    self.status().game_linked = true;
                    self.emit(LauncherEvent::GameLinked);
                    self.welcome_game();
                }
                TransportEvent::PeerDisconnected(..) => {
                    self.status().game_linked = false;
                    self.emit(LauncherEvent::GameUnlinked);
                }
                TransportEvent::Message { payload, .. } => self.on_game_payload(&payload),
            }
        }
    }

    fn on_game_payload(&mut self, payload: &[u8]) {
        if let Ok(Message::State(state)) = decode(payload)
            && self.remote_peer.is_some()
            && let Some(session) = self.remote.as_mut()
        {
            let _ = session.send_state(&state);
        }
    }

    fn pump_remote(&mut self, now: Instant) {
        let Some(session) = self.remote.as_mut() else {
            return;
        };
        let mut events = Vec::new();
        session.poll(now, &mut events);
        for event in events {
            self.on_remote_event(event);
        }
    }

    fn on_remote_event(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::PeerJoined { peer, name, .. } => {
                self.remote_peer = Some((peer, name.clone()));
                let mut status = self.status();
                status.state = ConnectionState::Connected;
                status.peer_name = Some(name.clone());
                drop(status);
                self.emit(LauncherEvent::PeerJoined { name });
                self.welcome_game();
            }
            SessionEvent::PeerState { state, .. } => {
                self.send_to_game(&Message::State(state));
            }
            SessionEvent::PeerLeft { .. } => self.on_peer_left(),
            SessionEvent::Rejected { reason } => {
                self.remote = None;
                self.forget_peer();
                self.last_reject = Some(reason);
                let mut status = self.status();
                status.state = ConnectionState::Rejected(reason);
                drop(status);
                self.send_to_game(&Message::Reject(rc_proto::Reject { reason }));
                self.emit(LauncherEvent::Rejected { reason });
            }
            SessionEvent::PeerRefused { reason, .. } => {
                self.emit(LauncherEvent::PeerRefused { reason });
            }
        }
    }

    fn on_peer_left(&mut self) {
        self.forget_peer();
        self.send_to_game(&Message::Bye);
        self.emit(LauncherEvent::PeerLeft);
        let hosting = self
            .remote
            .as_ref()
            .is_some_and(|session| session.local_slot() == Some(0));
        if hosting {
            self.status().state = ConnectionState::Hosting;
            return;
        }
        self.remote = None;
        self.status().state = ConnectionState::Failed("the host left".to_owned());
    }

    fn forget_peer(&mut self) {
        self.remote_peer = None;
        self.last_path_info = None;
        let mut status = self.status();
        status.peer_name = None;
        status.rtt = None;
        status.direct_path = None;
    }

    fn welcome_game(&mut self) {
        let Some((_, name)) = self.remote_peer.clone() else {
            if let Some(reason) = self.last_reject {
                self.send_to_game(&Message::Reject(rc_proto::Reject { reason }));
            }
            return;
        };
        let slot = self
            .remote
            .as_ref()
            .and_then(Session::local_slot)
            .unwrap_or(0);
        self.send_to_game(&Message::Welcome(Welcome {
            version: PROTOCOL_VERSION,
            slot,
            host_name: name,
        }));
    }

    fn send_path_info(&mut self, now: Instant) {
        let due = self
            .last_path_info
            .is_none_or(|at| now.saturating_duration_since(at) >= PATH_INFO_INTERVAL);
        let (Some((peer, _)), Some(session)) = (self.remote_peer.as_ref(), self.remote.as_ref())
        else {
            return;
        };
        if !due {
            return;
        }
        let rtt = session.rtt(*peer);
        let direct = session
            .transport()
            .link_stats(*peer)
            .map(|stats| stats.direct);
        self.last_path_info = Some(now);
        let mut status = self.status();
        status.rtt = rtt;
        status.direct_path = direct;
        drop(status);
        if let (Some(rtt), Some(direct)) = (rtt, direct) {
            let rtt_micros = u32::try_from(rtt.as_micros()).unwrap_or(u32::MAX);
            self.send_to_game(&Message::PathInfo(PathInfo { rtt_micros, direct }));
        }
    }

    fn send_to_game(&mut self, message: &Message) {
        let Ok(bytes) = encode(message) else {
            return;
        };
        let channel = if message.is_latest_wins() {
            Channel::UnreliableLatest
        } else {
            Channel::ReliableOrdered
        };
        let _ = self.link.send(LINK_GAME_PEER, channel, &bytes);
    }

    fn emit(&self, event: LauncherEvent) {
        let _ = self.events.send(event);
    }

    fn status(&self) -> MutexGuard<'_, LinkStatus> {
        self.status.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
