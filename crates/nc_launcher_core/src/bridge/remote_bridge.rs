use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use nc_net::link_traffic::{LinkTraffic, RefusalLog};
use nc_net::{
    Channel, Invite, LINK_GAME_PEER, LeaveReason, LocalLinkTransport, Role, Session, SessionConfig,
    SessionEvent, Transport, TransportEvent,
};
use nc_wire::{Envelope, decode_enveloped, encode_enveloped};
use replicant_game::readiness::LocalGameReadiness;
use replicant_proto::{
    GameReadiness, LobbyBody, LobbyReady, LobbyStart, Message, PROTOCOL_VERSION, PartnerBody,
    PathInfo, PlayerSettings, RejectReason, SessionOptions, Welcome, WireError,
};

use crate::bridge::members::Members;
use crate::event::options_differ_from_last;
use crate::{
    ConnectionState, INVALID_TICKET_TEXT, LauncherConfig, LauncherEvent, LinkStatus, NetError,
    PATH_INFO_INTERVAL, STORY_RULES, TransportFactory,
};

type RemoteSession = Session<Box<dyn Transport>, Message>;

pub(crate) struct Bridge {
    link: LocalLinkTransport,
    factory: TransportFactory,
    config: LauncherConfig,
    status: Arc<Mutex<LinkStatus>>,
    events: Sender<LauncherEvent>,
    remote: Option<RemoteSession>,
    members: Members,
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
    traffic: LinkTraffic,
    refusals: RefusalLog,
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
            members: Members::default(),
            last_path_info: None,
            last_reject: None,
            options: SessionOptions::default(),
            last_options_announced: None,
            settings: None,
            local_game: LocalGameReadiness::new(),
            readiness_sent: None,
            own_readiness_logged: None,
            own_lobby: None,
            own_body: None,
            traffic: LinkTraffic::default(),
            refusals: RefusalLog::default(),
        }
    }

    pub(crate) fn host(&mut self) -> Result<String, NetError> {
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

    pub(crate) fn join(&mut self, ticket: &str) -> Result<(), NetError> {
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

    pub(crate) fn stop(&mut self) {
        self.last_reject = None;
        let had_peer = !self.members.is_empty();
        if let Some(mut session) = self.remote.take() {
            session.close();
        }
        self.forget_peer();
        if had_peer {
            self.send_to_game(&Message::Bye);
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

    pub(crate) fn set_options(&mut self, options: SessionOptions) {
        self.options = options;
        if !self.status().guest {
            self.last_options_announced = Some(options);
        }
        self.send_options_if_hosting();
    }

    pub(crate) fn set_settings(&mut self, settings: PlayerSettings) {
        self.settings = Some(settings);
        self.send_to_game(&Message::Settings(settings));
    }

    pub(crate) fn set_lobby_ready(&mut self, ready: LobbyReady) {
        self.own_lobby = Some(ready);
        self.send_own_lobby();
    }

    pub(crate) fn set_lobby_body(&mut self, body: PartnerBody) {
        self.own_body = Some(body);
        self.send_own_body();
    }

    pub(crate) fn quit_game(&mut self) {
        self.send_to_game(&Message::QuitGame);
    }

    pub(crate) fn start_lobby(&mut self, mode: u8) {
        if self.status().guest {
            return;
        }
        if self.members.is_empty() {
            return;
        }
        let Some(session) = self.remote.as_mut() else {
            return;
        };
        let _ = session.send_relayed(&Message::LobbyStart(LobbyStart { mode }));
    }

    pub(crate) fn set_name(&mut self, name: &str) {
        self.config = LauncherConfig::new(name);
    }

    pub(crate) fn pump(&mut self, now: Instant) {
        self.pump_link();
        self.pump_remote(now);
        self.send_path_info(now);
        self.share_game_readiness();
    }

    fn new_session(&self, role: Role) -> Result<RemoteSession, NetError> {
        let transport = (self.factory)().map_err(|error| NetError::Failed(error.to_string()))?;
        Ok(Session::new(
            transport,
            SessionConfig {
                role,
                name: self.config.name.clone(),
                build: self.config.build,
                rules: STORY_RULES,
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
                        self.send_to_game(&Message::Settings(settings));
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
        let (envelope, message) = match decode_enveloped(payload) {
            Ok(framed) => framed,
            Err(error) => {
                self.report_refused("undecodable game message", &error);
                return;
            }
        };
        match message {
            Message::Readiness(readiness) => {
                self.local_game.game_reported(readiness);
                return;
            }
            Message::Settings(settings) => {
                self.settings = Some(settings);
                self.emit(LauncherEvent::GameChangedSettings { settings });
                return;
            }
            Message::SessionOptions(options) => {
                self.accept_options_from_game(options);
                return;
            }
            Message::LobbyReady(_)
            | Message::LobbyStart(_)
            | Message::LobbyBody(_)
            | Message::QuitGame => return,
            _ => {}
        }
        if !self.members.addresses_a_member(envelope.to) {
            return;
        }
        let Some(session) = self.remote.as_mut() else {
            return;
        };
        match message {
            state @ Message::State(_) => {
                if let Err(error) = session.send_ordered(&state) {
                    self.report_refused("State", &error);
                }
                self.traffic.note_sent("State", Instant::now());
            }
            event if event.is_relayed() => {
                if let Err(error) = session.send_relayed(&event) {
                    self.report_refused(event.kind_name(), &error);
                }
                self.traffic.note_sent(event.kind_name(), Instant::now());
            }
            _ => {}
        }
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

    fn on_remote_event(&mut self, event: SessionEvent<Message>) {
        if let SessionEvent::Message { from, .. } = &event
            && !self.members.is_member(*from)
        {
            return;
        }
        match event {
            SessionEvent::PlayerJoined { slot, name, peer } => {
                if !self.members.admit(slot, name.clone(), peer) {
                    return;
                }
                self.traffic.forget();
                self.readiness_sent = None;
                self.send_own_lobby();
                self.send_own_body();
                let mut status = self.status();
                status.state = ConnectionState::Connected;
                self.members.write_partner_status(&mut status);
                drop(status);
                self.emit(LauncherEvent::PeerJoined { name });
                self.welcome_game();
            }
            SessionEvent::Message {
                from,
                message: message @ Message::State(_),
            } => {
                self.traffic.note_received("State", Instant::now());
                self.relay_to_game(from, &message);
            }
            SessionEvent::Message { from, message } => {
                self.traffic
                    .note_received(message.kind_name(), Instant::now());
                match message {
                    Message::Readiness(readiness) => {
                        self.status().peer_readiness = readiness;
                        self.emit(LauncherEvent::PartnerGameReadiness { readiness });
                    }
                    Message::SessionOptions(options) => {
                        if options_differ_from_last(&mut self.last_options_announced, options) {
                            self.emit(LauncherEvent::SessionOptionsChanged { options });
                        }
                    }
                    Message::LobbyReady(ready) => {
                        self.status().peer_lobby = Some(ready);
                        self.emit(LauncherEvent::LobbyPartnerReady { ready });
                        return;
                    }
                    Message::LobbyBody(LobbyBody { body }) => {
                        self.status().peer_body = Some(body);
                        self.emit(LauncherEvent::LobbyPartnerBody { body });
                        return;
                    }
                    Message::LobbyStart(start) => {
                        if self.status().guest {
                            self.emit(LauncherEvent::LobbyStartReceived { mode: start.mode });
                        }
                        return;
                    }
                    _ => {}
                }
                self.relay_to_game(from, &message);
            }
            SessionEvent::PlayerLeft { slot, reason, .. } => self.on_peer_left(slot, reason),
            SessionEvent::Rejected { reason } => {
                self.remote = None;
                self.forget_peer();
                self.last_reject = Some(reason);
                let mut status = self.status();
                status.state = ConnectionState::Rejected(reason);
                drop(status);
                self.send_to_game(&Message::Reject(replicant_proto::Reject { reason }));
                self.emit(LauncherEvent::Rejected { reason });
            }
            SessionEvent::RulesChanged(rules) => {
                self.send_to_game(&Message::Rules(rules));
            }
            SessionEvent::PeerRefused { reason, .. } => {
                self.emit(LauncherEvent::PeerRefused { reason });
            }
        }
    }

    fn on_peer_left(&mut self, slot: u8, reason: LeaveReason) {
        if self.members.leave(slot).is_none() {
            return;
        }
        let hosting = self
            .remote
            .as_ref()
            .is_some_and(|session| session.local_slot() == Some(0));
        self.forget_peer();
        if hosting {
            self.send_to_game(&Message::MemberLeft { slot });
        } else {
            self.send_to_game(&Message::Bye);
        }
        let detail = format!(
            "{}; {}",
            reason.label(),
            self.traffic.describe(Instant::now())
        );
        self.emit(LauncherEvent::PeerLeft { detail });
        if hosting {
            self.status().state = ConnectionState::Hosting;
            return;
        }
        self.remote = None;
        self.status().state = ConnectionState::Failed("the host left".to_owned());
    }

    fn forget_peer(&mut self) {
        self.members.clear();
        self.last_path_info = None;
        self.readiness_sent = None;
        self.last_options_announced = None;
        let mut status = self.status();
        self.members.write_partner_status(&mut status);
    }

    fn welcome_game(&mut self) {
        let Some(partner) = self.members.partner().cloned() else {
            if let Some(reason) = self.last_reject {
                self.send_to_game(&Message::Reject(replicant_proto::Reject { reason }));
            }
            return;
        };
        self.send_to_game(&Message::Welcome(Welcome {
            version: PROTOCOL_VERSION,
            slot: self.own_slot(),
            host_name: partner.name.clone(),
            rules: self.remote.as_ref().map_or(STORY_RULES, Session::rules),
        }));
        if let Some(announcement) = self.members.member_announcement() {
            self.send_to_game(&announcement);
        }
        let peer_readiness = self.status().peer_readiness;
        self.relay_to_game(partner.slot, &Message::Readiness(peer_readiness));
    }

    fn send_own_lobby(&mut self) {
        if self.members.is_empty() {
            return;
        }
        let (Some(ready), Some(session)) = (self.own_lobby, self.remote.as_mut()) else {
            return;
        };
        let _ = session.send_relayed(&Message::LobbyReady(ready));
    }

    fn send_own_body(&mut self) {
        if self.members.is_empty() {
            return;
        }
        let (Some(body), Some(session)) = (self.own_body, self.remote.as_mut()) else {
            return;
        };
        let _ = session.send_relayed(&Message::LobbyBody(LobbyBody { body }));
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
        if self.members.is_empty() {
            return;
        }
        let Some(session) = self.remote.as_mut() else {
            return;
        };
        if session.send_relayed(&Message::Readiness(current)).is_ok() {
            self.readiness_sent = Some(current);
        }
    }

    fn send_path_info(&mut self, now: Instant) {
        let due = self
            .last_path_info
            .is_none_or(|at| now.saturating_duration_since(at) >= PATH_INFO_INTERVAL);
        let (Some(peer), Some(session)) = (
            self.members.partner().map(|partner| partner.peer),
            self.remote.as_ref(),
        ) else {
            return;
        };
        if !due {
            return;
        }
        let rtt = session.rtt(peer);
        let direct = session
            .transport()
            .link_stats(peer)
            .map(|stats| stats.direct);
        self.last_path_info = Some(now);
        self.send_options_if_hosting();
        let mut status = self.status();
        status.rtt = rtt;
        status.direct_path = direct;
        drop(status);
        if let (Some(rtt), Some(direct)) = (rtt, direct) {
            let rtt_micros = u32::try_from(rtt.as_micros()).unwrap_or(u32::MAX);
            self.send_to_game(&Message::PathInfo(PathInfo { rtt_micros, direct }));
        }
    }

    fn send_options_if_hosting(&mut self) {
        let hosting = self
            .remote
            .as_ref()
            .is_some_and(|session| session.local_slot() == Some(0));
        if !hosting || self.members.is_empty() {
            return;
        }
        let message = Message::SessionOptions(self.options);
        self.send_to_game(&message);
        if let Some(session) = self.remote.as_mut() {
            let _ = session.send_relayed(&message);
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

    fn own_slot(&self) -> u8 {
        self.remote
            .as_ref()
            .and_then(Session::local_slot)
            .unwrap_or(0)
    }

    fn send_to_game(&mut self, message: &Message) {
        self.relay_to_game(self.own_slot(), message);
    }

    fn relay_to_game(&mut self, from: u8, message: &Message) {
        let envelope = Envelope { from, to: None };
        let bytes = match encode_enveloped(&envelope, message) {
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
