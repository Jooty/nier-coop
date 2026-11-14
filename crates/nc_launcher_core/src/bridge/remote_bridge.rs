use std::collections::BTreeMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use nc_net::link_traffic::{LinkTraffic, RefusalLog};
use nc_net::{
    Channel, Invite, LINK_GAME_PEER, LeaveReason, LinkProgress, LocalLinkTransport, ProgressBoard,
    Role, Session, SessionConfig, SessionEvent, Transport, TransportEvent,
};
use nc_wire::{
    ENVELOPE_BYTES, Envelope, GameReadiness, LinkPayload, LobbyCharacter, LobbyPayload, LobbyReady,
    LobbyStart, Reject, RejectReason, SessionRules, Welcome, WireError,
    decode_enveloped_ignoring_trailing, encode_enveloped, frame_enveloped,
};
use replicant_game::readiness::LocalGameReadiness;
use replicant_proto::{
    CampaignCard, PartnerBody, PlayerSettings, SeatOutcome, SeatPart, SeatReceipt, SessionOptions,
};

use crate::bridge::members::Members;
use crate::bridge::{BridgeMessage, BridgeParts, GameBridge, LauncherPayload};
use crate::event::options_differ_from_last;
use crate::seat_transfer::AssemblerStep;
use crate::{
    ConnectionState, HOST_ANSWER_TIMEOUT, HOST_KEY_NOT_FOUND_TEXT, HOST_UNREACHABLE_TEXT,
    INVALID_TICKET_TEXT, LauncherConfig, LauncherEvent, LinkStatus, MemberChange, NetError,
    PATH_INFO_INTERVAL, SEAT_PARTS_PER_PUMP, STORY_RULES, SeatAssembler, SeatSender,
    SessionEndCause, TransportFactory,
};

type RemoteSession<M> = Session<Box<dyn Transport>, M>;

pub(crate) struct Bridge<M: BridgeMessage> {
    link: LocalLinkTransport,
    factory: TransportFactory,
    config: LauncherConfig,
    status: Arc<Mutex<LinkStatus>>,
    events: Sender<LauncherEvent>,
    remote: Option<RemoteSession<M>>,
    members: Members,
    last_path_info: Option<Instant>,
    last_reject: Option<RejectReason>,
    options: SessionOptions,
    rules: SessionRules,
    last_options_announced: Option<SessionOptions>,
    settings: Option<PlayerSettings>,
    local_game: LocalGameReadiness,
    readiness_sent: Option<GameReadiness>,
    own_readiness_logged: Option<GameReadiness>,
    own_lobby: Option<LobbyReady>,
    own_body: Option<PartnerBody>,
    own_character: Option<u8>,
    own_card: Option<CampaignCard>,
    incoming_seats: BTreeMap<u8, SeatAssembler>,
    outgoing_seats: BTreeMap<u8, SeatSender>,
    traffic: LinkTraffic,
    refusals: RefusalLog,
    trailing_bytes_said: bool,
    protocol_skew_said: bool,
    progress: ProgressBoard,
    progress_logged: usize,
    session_asked_at: Option<Instant>,
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
            members: Members::default(),
            last_path_info: None,
            last_reject: None,
            options: parts.options,
            rules: STORY_RULES,
            last_options_announced: None,
            settings: parts.settings,
            local_game: LocalGameReadiness::new(),
            readiness_sent: None,
            own_readiness_logged: None,
            own_lobby: parts.own_lobby,
            own_body: parts.own_body,
            own_character: parts.own_character,
            own_card: None,
            incoming_seats: BTreeMap::new(),
            outgoing_seats: BTreeMap::new(),
            traffic: LinkTraffic::default(),
            refusals: RefusalLog::default(),
            trailing_bytes_said: false,
            protocol_skew_said: false,
            progress: parts.progress,
            progress_logged: 0,
            session_asked_at: None,
        }
    }
}

impl<M: BridgeMessage> GameBridge for Bridge<M> {
    fn host(&mut self) -> Result<String, NetError> {
        self.end_session(SessionEndCause::Replaced);
        let mut session = self.new_session(Role::Host)?;
        let invite = session
            .host()
            .map_err(|error| NetError::Failed(error.to_string()))?;
        let ticket = invite.as_str().to_owned();
        let rules = session.rules();
        self.remote = Some(session);
        let mut status = self.status();
        status.rules = rules;
        status.guest = false;
        status.own_slot = Some(0);
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
        self.end_session(SessionEndCause::Replaced);
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

    fn end_session(&mut self, cause: SessionEndCause) {
        if self.remote.is_none() {
            return;
        }
        let last_seen = self.status().clone();
        self.stop();
        self.emit(LauncherEvent::SessionEnded { cause, last_seen });
    }

    fn stop(&mut self) {
        self.last_reject = None;
        self.session_asked_at = None;
        let slots: Vec<u8> = self.members.all().map(|member| member.slot).collect();
        if let Some(mut session) = self.remote.take() {
            session.close();
        }
        self.forget_peer();
        if !slots.is_empty() {
            self.send_to_game(&M::from_link(LinkPayload::Bye));
            let detail = format!(
                "you closed the link; {}",
                self.traffic.describe(Instant::now())
            );
            for slot in slots {
                self.emit(LauncherEvent::MemberLeft {
                    slot,
                    detail: detail.clone(),
                });
            }
        }
        let mut status = self.status();
        status.state = ConnectionState::Idle;
        status.guest = false;
        status.own_slot = None;
        status.ticket = None;
    }

    fn set_options(&mut self, options: SessionOptions) {
        self.options = options;
        if !self.status().guest {
            self.last_options_announced = Some(options);
        }
        self.send_options_if_hosting();
    }

    fn set_rules(&mut self, rules: SessionRules) {
        self.rules = rules;
        // Before a session exists the picked rules are what it will open with.
        self.status().rules = rules;
        if let Some(session) = self.remote.as_mut() {
            session.set_rules(rules);
            let applied = session.rules();
            self.status().rules = applied;
        }
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

    fn set_campaign_card(&mut self, card: CampaignCard) {
        if !M::CARRIES_CAMPAIGN_SAVES {
            return;
        }
        self.own_card = Some(card);
        self.send_own_card();
    }

    fn send_seat(&mut self, slot: u8, seat: Vec<u8>, transfer: u32) {
        if !M::CARRIES_CAMPAIGN_SAVES {
            return;
        }
        if !self.members.is_member(slot) {
            self.report_refused("SeatPart", &WireError::OutOfRange { field: "slot" });
            return;
        }
        match SeatSender::new(transfer, seat) {
            Ok(sender) => {
                self.outgoing_seats.insert(slot, sender);
            }
            Err(reason) => self.emit(LauncherEvent::SeatTransferRefused {
                slot,
                transfer,
                reason,
                by_peer: false,
            }),
        }
    }

    fn quit_game(&mut self) {
        self.send_launcher_payload_to_game(LauncherPayload::QuitGame);
    }

    fn start_lobby(&mut self, mode: u8) {
        if self.status().guest {
            return;
        }
        if self.members.is_empty() {
            return;
        }
        let Some(session) = self.remote.as_mut() else {
            return;
        };
        let _ = session.send_relayed(&M::from_lobby(LobbyPayload::Start(LobbyStart { mode })));
    }

    fn remove_member(&mut self, slot: u8) {
        if self.status().guest {
            return;
        }
        if let Some(session) = self.remote.as_mut() {
            session.remove(slot);
        }
    }

    fn set_name(&mut self, name: &str) {
        self.config = LauncherConfig {
            build: self.config.build,
            ..LauncherConfig::new(name)
        };
        if let Some(session) = self.remote.as_mut() {
            session.set_name(&self.config.name);
        }
    }

    fn pump(&mut self, now: Instant) {
        self.pump_link();
        self.pump_remote(now);
        self.log_progress();
        self.give_up_on_a_silent_host(now);
        self.send_path_info(now);
        self.share_game_readiness();
        self.send_seat_parts();
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
            progress: bridge.progress,
        }
    }
}

impl<M: BridgeMessage> Bridge<M> {
    fn new_session(&mut self, role: Role) -> Result<RemoteSession<M>, NetError> {
        let sink = self.progress.fresh_sink();
        self.progress_logged = 0;
        self.session_asked_at = Some(Instant::now());
        let transport = (self.factory)().map_err(|error| NetError::Failed(error.to_string()))?;
        let mut session = Session::new(
            transport,
            SessionConfig {
                role,
                name: self.config.name.clone(),
                build: self.config.build,
                rules: self.rules,
            },
        );
        session.report_progress(sink);
        Ok(session)
    }

    fn log_progress(&mut self) {
        let Some(asked_at) = self.session_asked_at else {
            return;
        };
        let entries = self.progress.entries();
        for entry in entries.iter().skip(self.progress_logged) {
            self.emit(LauncherEvent::LinkProgress {
                after: entry.at.saturating_duration_since(asked_at),
                line: entry.progress.describe(),
            });
        }
        self.progress_logged = entries.len();
    }

    fn give_up_on_a_silent_host(&mut self, now: Instant) {
        let joining = self.status().state == ConnectionState::Joining;
        let Some(asked_at) = self.session_asked_at.filter(|_| joining) else {
            return;
        };
        let entries = self.progress.entries();
        let key_not_found = entries
            .iter()
            .any(|entry| matches!(entry.progress, LinkProgress::HostNotFound { .. }));
        let dial_failed = entries
            .iter()
            .any(|entry| entry.progress == LinkProgress::DialFailed);
        let timed_out = now.saturating_duration_since(asked_at) >= HOST_ANSWER_TIMEOUT;
        if !key_not_found && !dial_failed && !timed_out {
            return;
        }
        let message = if key_not_found {
            HOST_KEY_NOT_FOUND_TEXT
        } else {
            HOST_UNREACHABLE_TEXT
        };
        self.end_session(SessionEndCause::HostUnreachable);
        self.status().state = ConnectionState::Failed(message.to_owned());
        self.emit(LauncherEvent::Failed {
            message: message.to_owned(),
        });
    }

    fn pump_link(&mut self) {
        let mut incoming = Vec::new();
        self.link.poll(&mut incoming);
        for event in incoming {
            match event {
                TransportEvent::PeerConnected(_) => {
                    self.trailing_bytes_said = false;
                    self.protocol_skew_said = false;
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
        let (envelope, message, extra) = match decode_enveloped_ignoring_trailing::<M>(payload) {
            Ok(framed) => framed,
            Err(error) => {
                self.report_refused("undecodable game message", &error);
                return;
            }
        };
        let original = (extra > 0).then(|| &payload[ENVELOPE_BYTES..]);
        if extra > 0 {
            self.say_trailing_bytes_once(extra);
        }
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
            Ok(LauncherPayload::GameSaved { succeeded }) => {
                self.emit(LauncherEvent::GameSaved { succeeded });
                return;
            }
            Ok(
                LauncherPayload::LobbyBody(_)
                | LauncherPayload::QuitGame
                | LauncherPayload::CampaignCard(_)
                | LauncherPayload::SeatPart(_)
                | LauncherPayload::SeatReceipt(_),
            ) => return,
            Err(message) => message,
        };
        let message = match message.into_link() {
            Ok(LinkPayload::Hello(hello)) => {
                self.check_game_protocol(hello.version);
                return;
            }
            Ok(_) => return,
            Err(message) => message,
        };
        if !self.members.addresses_a_member(envelope.to) {
            return;
        }
        let addressed = envelope.to.filter(|_| self.members.count() > 1);
        let Some(session) = self.remote.as_mut() else {
            return;
        };
        let kind = message.kind_name();
        let sent = if let Some(body) = original {
            session.send_unchanged(addressed, &message, body)
        } else if message.ordered_tick().is_some() {
            match addressed {
                Some(slot) => session.send_ordered_to(slot, &message),
                None => session.send_ordered(&message),
            }
        } else if message.is_relayed() {
            match addressed {
                Some(slot) => session.send_relayed_to(slot, &message),
                None => session.send_relayed(&message),
            }
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
        if let SessionEvent::Message { from, .. }
        | SessionEvent::MessageWithTrailingBytes { from, .. } = &event
            && !self.members.is_member(*from)
        {
            return;
        }
        match event {
            SessionEvent::PlayerJoined { slot, name, peer } => {
                let first_member = self.members.is_empty();
                self.members.admit(slot, name.clone(), peer);
                self.traffic.forget();
                self.readiness_sent = None;
                self.send_own_lobby();
                self.send_own_body();
                self.send_own_character();
                self.send_own_card();
                let mut status = self.status();
                status.state = ConnectionState::Connected;
                self.members.write_status(&mut status);
                if let Some(session) = self.remote.as_ref() {
                    status.rules = session.rules();
                    status.own_slot = session.local_slot();
                }
                drop(status);
                self.emit(LauncherEvent::MemberJoined { slot, name });
                if first_member {
                    self.welcome_game();
                } else if let Some(announcement) = self.members.member_announcement::<M>(slot) {
                    self.send_to_game(&announcement);
                }
            }
            SessionEvent::Message { from, message } => {
                self.traffic
                    .note_received(message.kind_name(), Instant::now());
                self.on_peer_relayed(from, message, None);
            }
            SessionEvent::MessageWithTrailingBytes {
                from,
                message,
                body,
                ..
            } => {
                self.traffic
                    .note_received(message.kind_name(), Instant::now());
                self.on_peer_relayed(from, message, Some(&body));
            }
            SessionEvent::PlayerLeft { slot, reason, .. } => self.on_peer_left(slot, reason),
            SessionEvent::Rejected { reason } => {
                let last_seen = self.status().clone();
                self.remote = None;
                self.forget_peer();
                self.last_reject = Some(reason);
                let mut status = self.status();
                status.state = ConnectionState::Rejected(reason);
                drop(status);
                self.send_to_game(&M::from_link(LinkPayload::Reject(Reject { reason })));
                self.emit(LauncherEvent::Rejected { reason });
                self.emit(LauncherEvent::SessionEnded {
                    cause: SessionEndCause::Refused(reason),
                    last_seen,
                });
            }
            SessionEvent::RulesChanged(rules) => {
                self.status().rules = rules;
                self.send_to_game(&M::from_link(LinkPayload::Rules(rules)));
            }
            SessionEvent::MemberPings(pings) => {
                let mut status = self.status();
                for ping in pings {
                    if let Some(member) = status.members.iter_mut().find(|m| m.slot == ping.slot) {
                        member.rtt_ms = Some(ping.rtt_ms);
                    }
                }
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

    fn on_peer_relayed(&mut self, from: u8, message: M, original: Option<&[u8]>) {
        let message = match message.into_lobby() {
            Ok(LobbyPayload::GameReadiness(readiness)) => {
                self.change_member(from, MemberChange::Readiness(readiness));
                self.relay_to_game(from, &M::from_lobby(LobbyPayload::GameReadiness(readiness)));
                return;
            }
            Ok(LobbyPayload::Ready(ready)) => {
                self.change_member(from, MemberChange::Lobby(ready));
                return;
            }
            Ok(LobbyPayload::Character(LobbyCharacter { index })) => {
                self.change_member(from, MemberChange::Character(index));
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
                self.relay_launcher_payload_to_game(from, LauncherPayload::SessionOptions(options));
            }
            Ok(LauncherPayload::LobbyBody(body)) => {
                self.change_member(from, MemberChange::Body(body));
            }
            Ok(LauncherPayload::CampaignCard(card)) => {
                if self.status().guest && from == 0 {
                    self.emit(LauncherEvent::CampaignCardReceived { card });
                }
            }
            Ok(LauncherPayload::SeatPart(part)) => self.on_seat_part(from, &part),
            Ok(LauncherPayload::SeatReceipt(receipt)) => self.on_seat_receipt(from, receipt),
            Ok(LauncherPayload::GameSaved { .. }) => {}
            Ok(other) => self.relay_launcher_payload_to_game(from, other),
            Err(message) => self.relay_to_game_as_received(from, &message, original),
        }
    }

    fn on_peer_left(&mut self, slot: u8, reason: LeaveReason) {
        let last_seen = self.status().clone();
        if self.members.leave(slot).is_none() {
            return;
        }
        self.incoming_seats.remove(&slot);
        self.outgoing_seats.remove(&slot);
        let hosting = self
            .remote
            .as_ref()
            .is_some_and(|session| session.local_slot() == Some(0));
        let host_left = !hosting && slot == 0;
        if host_left {
            self.members.clear();
        }
        let others_remain = !self.members.is_empty();
        if !others_remain {
            self.forget_peer();
        }
        if host_left {
            self.send_to_game(&M::from_link(LinkPayload::Bye));
        } else {
            self.send_to_game(&M::from_link(LinkPayload::MemberLeft { slot }));
        }
        let detail = format!(
            "{}; {}",
            reason.label(),
            self.traffic.describe(Instant::now())
        );
        self.emit(LauncherEvent::MemberLeft { slot, detail });
        if others_remain {
            let mut status = self.status();
            self.members.write_status(&mut status);
            return;
        }
        if hosting {
            self.status().state = ConnectionState::Hosting;
            return;
        }
        self.remote = None;
        self.status().own_slot = None;
        let cause = SessionEndCause::from_host_leaving(reason);
        let message = cause.reason_line().unwrap_or_default();
        self.status().state = ConnectionState::Failed(message.to_owned());
        self.emit(LauncherEvent::SessionEnded { cause, last_seen });
    }

    fn forget_peer(&mut self) {
        self.members.clear();
        self.incoming_seats.clear();
        self.outgoing_seats.clear();
        self.last_path_info = None;
        self.readiness_sent = None;
        self.last_options_announced = None;
        let mut status = self.status();
        self.members.write_status(&mut status);
    }

    fn change_member(&mut self, slot: u8, change: MemberChange) {
        let mut status = self.status();
        if let Some(member) = status.members.iter_mut().find(|row| row.slot == slot) {
            member.apply(change);
        }
        drop(status);
        self.emit(LauncherEvent::MemberChanged { slot, change });
    }

    fn welcome_game(&mut self) {
        let Some(first) = self.members.first().cloned() else {
            if let Some(reason) = self.last_reject {
                self.send_to_game(&M::from_link(LinkPayload::Reject(Reject { reason })));
            }
            return;
        };
        self.send_to_game(&M::from_link(LinkPayload::Welcome(Welcome {
            version: M::PROTOCOL_VERSION,
            slot: self.own_slot(),
            host_name: first.name.clone(),
            rules: self.remote.as_ref().map_or(self.rules, Session::rules),
        })));
        let slots: Vec<u8> = self.members.all().map(|member| member.slot).collect();
        for slot in slots {
            if let Some(announcement) = self.members.member_announcement::<M>(slot) {
                self.send_to_game(&announcement);
            }
        }
        let first_readiness = self
            .status()
            .member(first.slot)
            .map_or(GameReadiness::NotLinked, |member| member.readiness);
        self.relay_to_game(
            first.slot,
            &M::from_lobby(LobbyPayload::GameReadiness(first_readiness)),
        );
    }

    fn send_own_lobby(&mut self) {
        if self.members.is_empty() {
            return;
        }
        let (Some(ready), Some(session)) = (self.own_lobby, self.remote.as_mut()) else {
            return;
        };
        let _ = session.send_relayed(&M::from_lobby(LobbyPayload::Ready(ready)));
    }

    fn send_own_body(&mut self) {
        if self.members.is_empty() {
            return;
        }
        let (Some(body), Some(session)) = (self.own_body, self.remote.as_mut()) else {
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
        if self.members.is_empty() {
            return;
        }
        let (Some(index), Some(session)) = (self.own_character, self.remote.as_mut()) else {
            return;
        };
        let character = LobbyPayload::Character(LobbyCharacter { index });
        let _ = session.send_relayed(&M::from_lobby(character));
    }

    fn send_own_card(&mut self) {
        if self.members.is_empty() {
            return;
        }
        let (Some(card), Some(session)) = (self.own_card.clone(), self.remote.as_mut()) else {
            return;
        };
        if let Some(message) = M::from_launcher(LauncherPayload::CampaignCard(card)) {
            let _ = session.send_relayed(&message);
        }
    }

    fn send_seat_parts(&mut self) {
        let Some(session) = self.remote.as_mut() else {
            return;
        };
        let mut events = Vec::new();
        for (slot, sender) in &mut self.outgoing_seats {
            let batch = sender.next_parts(SEAT_PARTS_PER_PUMP);
            if batch.is_empty() {
                continue;
            }
            for part in batch {
                if let Some(message) = M::from_launcher(LauncherPayload::SeatPart(part)) {
                    let _ = session.send_relayed_to(*slot, &message);
                }
            }
            events.push(LauncherEvent::SeatProgress {
                slot: *slot,
                transfer: sender.transfer(),
                parts_done: sender.parts_sent(),
                parts: sender.parts(),
                incoming: false,
            });
        }
        for event in events {
            self.emit(event);
        }
    }

    fn on_seat_part(&mut self, from: u8, part: &SeatPart) {
        let step = self.incoming_seats.entry(from).or_default().accept(part);
        let outcome = match step {
            AssemblerStep::Progress { parts_done, parts } => {
                self.emit(LauncherEvent::SeatProgress {
                    slot: from,
                    transfer: part.transfer,
                    parts_done,
                    parts,
                    incoming: true,
                });
                return;
            }
            AssemblerStep::Ignored => return,
            AssemblerStep::Whole(seat) => {
                self.emit(LauncherEvent::SeatReceived {
                    slot: from,
                    transfer: part.transfer,
                    seat,
                });
                SeatOutcome::ReceivedWholeAndValid
            }
            AssemblerStep::Refused(reason) => {
                self.emit(LauncherEvent::SeatTransferRefused {
                    slot: from,
                    transfer: part.transfer,
                    reason,
                    by_peer: false,
                });
                SeatOutcome::Refused(reason)
            }
        };
        let receipt = LauncherPayload::SeatReceipt(SeatReceipt {
            transfer: part.transfer,
            outcome,
        });
        let Some(receipt) = M::from_launcher(receipt) else {
            return;
        };
        if let Some(session) = self.remote.as_mut() {
            let _ = session.send_relayed_to(from, &receipt);
        }
    }

    fn on_seat_receipt(&mut self, from: u8, receipt: SeatReceipt) {
        let answers_current = self
            .outgoing_seats
            .get(&from)
            .is_some_and(|sender| sender.transfer() == receipt.transfer);
        if !answers_current {
            return;
        }
        self.outgoing_seats.remove(&from);
        self.emit(match receipt.outcome {
            SeatOutcome::ReceivedWholeAndValid => LauncherEvent::SeatDelivered {
                slot: from,
                transfer: receipt.transfer,
            },
            SeatOutcome::Refused(reason) => LauncherEvent::SeatTransferRefused {
                slot: from,
                transfer: receipt.transfer,
                reason,
                by_peer: true,
            },
        });
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
        let readiness = M::from_lobby(LobbyPayload::GameReadiness(current));
        if session.send_relayed(&readiness).is_ok() {
            self.readiness_sent = Some(current);
        }
    }

    fn send_path_info(&mut self, now: Instant) {
        let due = self
            .last_path_info
            .is_none_or(|at| now.saturating_duration_since(at) >= PATH_INFO_INTERVAL);
        let (Some(peer), Some(session)) = (
            self.members.first().map(|member| member.peer),
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
        self.measure_member_pings();
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

    fn measure_member_pings(&mut self) {
        let hosting = self
            .remote
            .as_ref()
            .is_some_and(|session| session.local_slot() == Some(0));
        let Some(session) = self.remote.as_ref().filter(|_| hosting) else {
            return;
        };
        let measured: Vec<(u8, Option<u32>)> = self
            .members
            .all()
            .map(|member| {
                let rtt_ms = session
                    .rtt(member.peer)
                    .map(|rtt| u32::try_from(rtt.as_millis()).unwrap_or(u32::MAX));
                (member.slot, rtt_ms)
            })
            .collect();
        let mut status = self.status();
        for (slot, rtt_ms) in measured {
            if let Some(member) = status.members.iter_mut().find(|row| row.slot == slot) {
                member.rtt_ms = rtt_ms.or(member.rtt_ms);
            }
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

    fn relay_launcher_payload_to_game(&mut self, from: u8, payload: LauncherPayload) {
        if let Some(message) = M::from_launcher(payload) {
            self.relay_to_game(from, &message);
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

    fn send_to_game(&mut self, message: &M) {
        self.relay_to_game(self.own_slot(), message);
    }

    fn relay_to_game(&mut self, from: u8, message: &M) {
        self.relay_to_game_as_received(from, message, None);
    }

    fn relay_to_game_as_received(&mut self, from: u8, message: &M, original: Option<&[u8]>) {
        let envelope = Envelope { from, to: None };
        let framed = match original {
            Some(body) => frame_enveloped(&envelope, body),
            None => encode_enveloped(&envelope, message),
        };
        let bytes = match framed {
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

    fn say_trailing_bytes_once(&mut self, extra: usize) {
        if self.trailing_bytes_said {
            return;
        }
        self.trailing_bytes_said = true;
        self.emit(LauncherEvent::BuildsDiffer {
            text: format!(
                "relayed a game message with {extra} trailing bytes: the game's hook is a newer build than this launcher (launcher protocol {}); restart the launcher",
                M::PROTOCOL_VERSION
            ),
        });
    }

    fn check_game_protocol(&mut self, game_protocol: u16) {
        if game_protocol == M::PROTOCOL_VERSION || self.protocol_skew_said {
            return;
        }
        self.protocol_skew_said = true;
        self.emit(LauncherEvent::BuildsDiffer {
            text: format!(
                "the game speaks protocol {game_protocol}, this launcher {}: restart the launcher after a build update",
                M::PROTOCOL_VERSION
            ),
        });
    }

    fn emit(&self, event: LauncherEvent) {
        let _ = self.events.send(event);
    }

    fn status(&self) -> MutexGuard<'_, LinkStatus> {
        self.status.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
