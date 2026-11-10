use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use nc_net::link_traffic::{LinkTraffic, RefusalLog};
use nc_net::{
    Channel, Invite, LINK_GAME_PEER, LeaveReason, LocalLinkTransport, PeerId, Role, Session,
    SessionConfig, SessionEvent, Transport, TransportEvent,
};
use nc_wire::{
    GameReadiness, LinkPayload, LobbyCharacter, LobbyPayload, LobbyReady, LobbyStart, Reject,
    RejectReason, Welcome, WireError, decode, encode,
};
use replicant_game::readiness::LocalGameReadiness;
use replicant_proto::{PartnerBody, PlayerSettings, SessionOptions};

use crate::bridge::{BridgeMessage, BridgeParts, GameBridge, LauncherPayload};
use crate::event::options_differ_from_last;
use crate::{
    ConnectionState, INVALID_TICKET_TEXT, LauncherConfig, LauncherEvent, LinkStatus, NetError,
    PATH_INFO_INTERVAL, TransportFactory,
};

type RemoteSession<M> = Session<Box<dyn Transport>, M>;

pub(crate) struct Bridge<M: BridgeMessage> {
    link: LocalLinkTransport,
    factory: TransportFactory,
    config: LauncherConfig,
    status: Arc<Mutex<LinkStatus>>,
    events: Sender<LauncherEvent>,
    remote: Option<RemoteSession<M>>,
    remote_peer: Option<(PeerId, String)>,
    last_path_info: Option<Instant>,
    last_reject: Option<RejectReason>,
    options: SessionOptions,
    last_options_announced: Option<SessionOptions>,
    settings: Option<PlayerSettings>,
    local_game: LocalGameReadiness,
    readiness_sent: Option<GameReadiness>,
    own_readiness_logged: Option<GameReadiness>,
    own_lobby: Option<LobbyReady>,
    own_body: Option<PartnerBody>,
    own_character: Option<u8>,
    traffic: LinkTraffic,
    refusals: RefusalLog,
}

impl<M: BridgeMessage> Bridge<M> {
    pub(crate) fn new(parts: BridgeParts) -> Self {
        Self {
            link: parts.link,
            factory: parts.factory,
            config: parts.config,
            status: parts.status,
            events: parts.events,
            remote: None,
            remote_peer: None,
            last_path_info: None,
            last_reject: None,
            options: parts.options,
            last_options_announced: None,
            settings: parts.settings,
            local_game: LocalGameReadiness::new(),
            readiness_sent: None,
            own_readiness_logged: None,
            own_lobby: parts.own_lobby,
            own_body: parts.own_body,
            own_character: parts.own_character,
            traffic: LinkTraffic::default(),
            refusals: RefusalLog::default(),
        }
    }
}

impl<M: BridgeMessage> GameBridge for Bridge<M> {
    fn host(&mut self) -> Result<String, NetError> {
        self.stop();
        let mut session = self.new_session(Role::Host)?;
        let invite = session
            .host()
            .map_err(|error| NetError::Failed(error.to_string()))?;
        let ticket = invite.as_str().to_owned();
        self.remote = Some(session);
        let mut status = self.status();
        status.guest = false;
        status.state = ConnectionState::Hosting;
        status.ticket = Some(ticket.clone());
        drop(status);
        self.emit(LauncherEvent::Hosting {
            ticket: ticket.clone(),
        });
        Ok(ticket)
    }

    fn join(&mut self, ticket: &str) -> Result<(), NetError> {
        let invite = Invite::new(ticket.trim().to_owned())
            .map_err(|_| NetError::Failed(INVALID_TICKET_TEXT.to_owned()))?;
        self.stop();
        let mut session = self.new_session(Role::Guest)?;
        session
            .join(&invite)
            .map_err(|_| NetError::Failed(INVALID_TICKET_TEXT.to_owned()))?;
        self.remote = Some(session);
        let mut status = self.status();
        status.guest = true;
        status.state = ConnectionState::Joining;
        drop(status);
        self.emit(LauncherEvent::Joining);
        Ok(())
    }

    fn stop(&mut self) {
        self.last_reject = None;
        let had_peer = self.remote_peer.is_some();
        if let Some(mut session) = self.remote.take() {
            session.close();
        }
        self.forget_peer();
        if had_peer {
            self.send_to_game(&M::from_link(LinkPayload::Bye));
            let detail = format!(
                "you closed the link; {}",
                self.traffic.describe(Instant::now())
            );
            self.emit(LauncherEvent::PeerLeft { detail });
        }
        let mut status = self.status();
        status.state = ConnectionState::Idle;
        status.guest = false;
        status.ticket = None;
    }

    fn set_options(&mut self, options: SessionOptions) {
        self.options = options;
        if !self.status().guest {
            self.last_options_announced = Some(options);
        }
        self.send_options_if_hosting();
    }

    fn set_settings(&mut self, settings: PlayerSettings) {
        self.settings = Some(settings);
        self.send_launcher_payload_to_game(LauncherPayload::Settings(settings));
    }

    fn set_lobby_ready(&mut self, ready: LobbyReady) {
        self.own_lobby = Some(ready);
        self.send_own_lobby();
    }

    fn set_lobby_body(&mut self, body: PartnerBody) {
        self.own_body = Some(body);
        self.send_own_body();
    }

    fn set_lobby_character(&mut self, index: u8) {
        self.own_character = Some(index);
        self.send_own_character();
    }

    fn quit_game(&mut self) {
        self.send_launcher_payload_to_game(LauncherPayload::QuitGame);
    }

    fn start_lobby(&mut self, mode: u8) {
        if self.status().guest {
            return;
        }
        let (Some(_), Some(session)) = (self.remote_peer.as_ref(), self.remote.as_mut()) else {
            return;
        };
        let _ = session.send_relayed(&M::from_lobby(LobbyPayload::Start(LobbyStart { mode })));
    }

    fn set_name(&mut self, name: &str) {
        self.config = LauncherConfig {
            build: self.config.build,
            ..LauncherConfig::new(name)
        };
    }

    fn pump(&mut self, now: Instant) {
        self.pump_link();
        self.pump_remote(now);
        self.send_path_info(now);
        self.share_game_readiness();
    }

    fn into_parts(mut self: Box<Self>) -> BridgeParts {
        self.stop();
        let bridge = *self;
        BridgeParts {
            link: bridge.link,
            factory: bridge.factory,
            config: bridge.config,
            status: bridge.status,
            events: bridge.events,
            options: bridge.options,
            settings: bridge.settings,
            own_lobby: bridge.own_lobby,
            own_body: bridge.own_body,
            own_character: bridge.own_character,
        }
    }
}

impl<M: BridgeMessage> Bridge<M> {
    fn new_session(&self, role: Role) -> Result<RemoteSession<M>, NetError> {
        let transport = (self.factory)().map_err(|error| NetError::Failed(error.to_string()))?;
        Ok(Session::new(
            transport,
            SessionConfig {
                role,
                name: self.config.name.clone(),
                build: self.config.build,
            },
        ))
    }

    fn pump_link(&mut self) {
        let mut incoming = Vec::new();
        self.link.poll(&mut incoming);
        for event in incoming {
            match event {
                TransportEvent::PeerConnected(_) => {
                    self.local_game.game_linked();
                    self.status().game_linked = true;
                    self.emit(LauncherEvent::GameLinked);
                    self.welcome_game();
                    if let Some(settings) = self.settings {
                        self.send_launcher_payload_to_game(LauncherPayload::Settings(settings));
                    }
                }
                TransportEvent::PeerDisconnected(_, reason) => {
                    self.local_game.game_unlinked();
                    self.status().game_linked = false;
                    let detail = format!(
                        "link {}; {}",
                        reason.label(),
                        self.traffic.describe(Instant::now())
                    );
                    self.emit(LauncherEvent::GameUnlinked { detail });
                }
                TransportEvent::Message { payload, .. } => self.on_game_payload(&payload),
            }
        }
    }

    fn on_game_payload(&mut self, payload: &[u8]) {
        let message = match decode::<M>(payload) {
            Ok(message) => message,
            Err(error) => {
                self.report_refused("undecodable game message", &error);
                return;
            }
        };
        let message = match message.into_lobby() {
            Ok(LobbyPayload::GameReadiness(readiness)) => {
                self.local_game.game_reported(readiness);
                return;
            }
            Ok(LobbyPayload::Ready(_) | LobbyPayload::Start(_) | LobbyPayload::Character(_)) => {
                return;
            }
            Err(message) => message,
        };
        let message = match message.into_launcher() {
            Ok(LauncherPayload::Settings(settings)) => {
                self.settings = Some(settings);
                self.emit(LauncherEvent::GameChangedSettings { settings });
                return;
            }
            Ok(LauncherPayload::SessionOptions(options)) => {
                self.accept_options_from_game(options);
                return;
            }
            Ok(LauncherPayload::LobbyBody(_) | LauncherPayload::QuitGame) => return,
            Err(message) => message,
        };
        if self.remote_peer.is_none() {
            return;
        }
        let Some(session) = self.remote.as_mut() else {
            return;
        };
        let kind = message.kind_name();
        let sent = if message.ordered_tick().is_some() {
            session.send_ordered(&message)
        } else if message.is_relayed() {
            session.send_relayed(&message)
        } else {
            return;
        };
        if let Err(error) = sent {
            self.report_refused(kind, &error);
        }
        self.traffic.note_sent(kind, Instant::now());
    }

    fn accept_options_from_game(&mut self, options: SessionOptions) {
        if self.status().guest {
            return;
        }
        self.options = options;
        if options_differ_from_last(&mut self.last_options_announced, options) {
            self.emit(LauncherEvent::SessionOptionsChanged { options });
        }
        self.send_options_if_hosting();
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

    fn on_remote_event(&mut self, event: SessionEvent<M>) {
        match event {
            SessionEvent::PeerJoined { peer, name, .. } => {
                self.traffic.forget();
                self.remote_peer = Some((peer, name.clone()));
                self.readiness_sent = None;
                self.send_own_lobby();
                self.send_own_body();
                self.send_own_character();
                let mut status = self.status();
                status.state = ConnectionState::Connected;
                status.peer_name = Some(name.clone());
                drop(status);
                self.emit(LauncherEvent::PeerJoined { name });
                self.welcome_game();
            }
            SessionEvent::PeerOrdered { message, .. } => {
                self.traffic
                    .note_received(message.kind_name(), Instant::now());
                self.send_to_game(&message);
            }
            SessionEvent::PeerRelayed { message, .. } => {
                self.traffic
                    .note_received(message.kind_name(), Instant::now());
                self.on_peer_relayed(message);
            }
            SessionEvent::PeerLeft { reason, .. } => self.on_peer_left(reason),
            SessionEvent::Rejected { reason } => {
                self.remote = None;
                self.forget_peer();
                self.last_reject = Some(reason);
                let mut status = self.status();
                status.state = ConnectionState::Rejected(reason);
                drop(status);
                self.send_to_game(&M::from_link(LinkPayload::Reject(Reject { reason })));
                self.emit(LauncherEvent::Rejected { reason });
            }
            SessionEvent::PeerRefused { reason, .. } => {
                self.emit(LauncherEvent::PeerRefused { reason });
                let their_version = self.remote.as_ref().and_then(Session::refused_peer_version);
                if let (RejectReason::VersionMismatch, Some(theirs)) = (reason, their_version) {
                    self.emit(LauncherEvent::PeerOnOtherVersion {
                        ours: M::PROTOCOL_VERSION,
                        theirs,
                    });
                }
            }
        }
    }

    fn on_peer_relayed(&mut self, message: M) {
        let message = match message.into_lobby() {
            Ok(LobbyPayload::GameReadiness(readiness)) => {
                self.status().peer_readiness = readiness;
                self.emit(LauncherEvent::PartnerGameReadiness { readiness });
                self.send_to_game(&M::from_lobby(LobbyPayload::GameReadiness(readiness)));
                return;
            }
            Ok(LobbyPayload::Ready(ready)) => {
                self.status().peer_lobby = Some(ready);
                self.emit(LauncherEvent::LobbyPartnerReady { ready });
                return;
            }
            Ok(LobbyPayload::Character(LobbyCharacter { index })) => {
                self.status().peer_character = Some(index);
                self.emit(LauncherEvent::LobbyPartnerCharacter { index });
                return;
            }
            Ok(LobbyPayload::Start(start)) => {
                if self.status().guest {
                    self.emit(LauncherEvent::LobbyStartReceived { mode: start.mode });
                }
                return;
            }
            Err(message) => message,
        };
        match message.into_launcher() {
            Ok(LauncherPayload::SessionOptions(options)) => {
                if options_differ_from_last(&mut self.last_options_announced, options) {
                    self.emit(LauncherEvent::SessionOptionsChanged { options });
                }
                self.send_launcher_payload_to_game(LauncherPayload::SessionOptions(options));
            }
            Ok(LauncherPayload::LobbyBody(body)) => {
                self.status().peer_body = Some(body);
                self.emit(LauncherEvent::LobbyPartnerBody { body });
            }
            Ok(other) => self.send_launcher_payload_to_game(other),
            Err(message) => self.send_to_game(&message),
        }
    }

    fn on_peer_left(&mut self, reason: LeaveReason) {
        self.forget_peer();
        self.send_to_game(&M::from_link(LinkPayload::Bye));
        let detail = format!(
            "{}; {}",
            reason.label(),
            self.traffic.describe(Instant::now())
        );
        self.emit(LauncherEvent::PeerLeft { detail });
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
        self.readiness_sent = None;
        self.last_options_announced = None;
        let mut status = self.status();
        status.peer_readiness = GameReadiness::NotLinked;
        status.peer_lobby = None;
        status.peer_body = None;
        status.peer_character = None;
        status.peer_name = None;
        status.rtt = None;
        status.direct_path = None;
    }

    fn welcome_game(&mut self) {
        let Some((_, name)) = self.remote_peer.clone() else {
            if let Some(reason) = self.last_reject {
                self.send_to_game(&M::from_link(LinkPayload::Reject(Reject { reason })));
            }
            return;
        };
        let slot = self
            .remote
            .as_ref()
            .and_then(Session::local_slot)
            .unwrap_or(0);
        self.send_to_game(&M::from_link(LinkPayload::Welcome(Welcome {
            version: M::PROTOCOL_VERSION,
            slot,
            host_name: name,
        })));
        let peer_readiness = self.status().peer_readiness;
        self.send_to_game(&M::from_lobby(LobbyPayload::GameReadiness(peer_readiness)));
    }

    fn send_own_lobby(&mut self) {
        let (Some(ready), Some(_), Some(session)) = (
            self.own_lobby,
            self.remote_peer.as_ref(),
            self.remote.as_mut(),
        ) else {
            return;
        };
        let _ = session.send_relayed(&M::from_lobby(LobbyPayload::Ready(ready)));
    }

    fn send_own_body(&mut self) {
        let (Some(body), Some(_), Some(session)) = (
            self.own_body,
            self.remote_peer.as_ref(),
            self.remote.as_mut(),
        ) else {
            return;
        };
        if let Some(message) = M::from_launcher(LauncherPayload::LobbyBody(body)) {
            let _ = session.send_relayed(&message);
        }
    }

    fn send_own_character(&mut self) {
        if !M::LOBBY_PICKS_CHARACTERS {
            return;
        }
        let (Some(index), Some(_), Some(session)) = (
            self.own_character,
            self.remote_peer.as_ref(),
            self.remote.as_mut(),
        ) else {
            return;
        };
        let character = LobbyPayload::Character(LobbyCharacter { index });
        let _ = session.send_relayed(&M::from_lobby(character));
    }

    fn share_game_readiness(&mut self) {
        let current = self.local_game.current();
        if self.own_readiness_logged != Some(current) {
            self.own_readiness_logged = Some(current);
            self.emit(LauncherEvent::OwnGameReadiness { readiness: current });
        }
        if self.readiness_sent == Some(current) {
            return;
        }
        let (Some(_), Some(session)) = (self.remote_peer.as_ref(), self.remote.as_mut()) else {
            return;
        };
        let readiness = M::from_lobby(LobbyPayload::GameReadiness(current));
        if session.send_relayed(&readiness).is_ok() {
            self.readiness_sent = Some(current);
        }
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
        self.send_options_if_hosting();
        let mut status = self.status();
        status.rtt = rtt;
        status.direct_path = direct;
        drop(status);
        if let (Some(rtt), Some(direct)) = (rtt, direct) {
            let rtt_micros = u32::try_from(rtt.as_micros()).unwrap_or(u32::MAX);
            self.send_to_game(&M::path_info(rtt_micros, direct));
        }
    }

    fn send_options_if_hosting(&mut self) {
        let hosting = self
            .remote
            .as_ref()
            .is_some_and(|session| session.local_slot() == Some(0));
        if !hosting || self.remote_peer.is_none() {
            return;
        }
        let Some(message) = M::from_launcher(LauncherPayload::SessionOptions(self.options)) else {
            return;
        };
        self.send_to_game(&message);
        if let Some(session) = self.remote.as_mut() {
            let _ = session.send_relayed(&message);
        }
    }

    fn send_launcher_payload_to_game(&mut self, payload: LauncherPayload) {
        if let Some(message) = M::from_launcher(payload) {
            self.send_to_game(&message);
        }
    }

    fn report_refused(&mut self, kind: &'static str, error: &WireError) {
        if self.refusals.should_log(kind, Instant::now()) {
            self.emit(LauncherEvent::MessageRefused {
                kind: kind.to_owned(),
                reason: error.to_string(),
            });
        }
    }

    fn send_to_game(&mut self, message: &M) {
        let bytes = match encode(message) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.report_refused(message.kind_name(), &error);
                return;
            }
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
