use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use nc_net::{LocalLinkTransport, Transport};
use nc_wire::{BuildGate, SessionRules};
use replicant_proto::{LobbyReady, PartnerBody, PlayerSettings, SessionOptions};

use crate::bridge::{BridgeParts, GameBridge, GameWire};
use crate::profile::GameProfile;
use crate::report::judge_rejection_text;
use crate::{
    COMMAND_REPLY_TIMEOUT, ConnectionReport, ConnectionState, LauncherConfig, LauncherEvent,
    LauncherNet, LinkStatus, NetError, NetState, NetStatus, TransportFactory, WORKER_WAKE_INTERVAL,
};

enum Command {
    Host {
        reply: Sender<Result<String, NetError>>,
    },
    Join {
        ticket: String,
        reply: Sender<Result<(), NetError>>,
    },
    Leave,
    SetOptions(SessionOptions),
    SetRules(SessionRules),
    SetSettings(PlayerSettings),
    SetName(String),
    LobbyReady(LobbyReady),
    LobbyBody(PartnerBody),
    LobbyCharacter(u8),
    StartLobby(u8),
    RemoveMember(u8),
    QuitGame,
    SetGame(GameWire),
}

pub struct LauncherCore {
    commands: Mutex<Option<Sender<Command>>>,
    events: Mutex<Receiver<LauncherEvent>>,
    status: Arc<Mutex<LinkStatus>>,
    factory: TransportFactory,
    config: LauncherConfig,
    link_port: u16,
    probe_session: Mutex<(GameWire, BuildGate)>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl LauncherCore {
    pub fn new(config: LauncherConfig, factory: TransportFactory) -> Result<Self, NetError> {
        let link = LocalLinkTransport::launcher(Some(config.build));
        Self::start(GameWire::Replicant, link, config, factory)
    }

    pub fn for_game(
        profile: &GameProfile,
        name: &str,
        factory: TransportFactory,
    ) -> Result<Self, NetError> {
        let wire = GameWire::of(profile);
        let link = LocalLinkTransport::launcher(None);
        link.accept_only_builds(&wire.hook_builds());
        Self::start(wire, link, LauncherConfig::for_game(profile, name), factory)
    }

    /// Carries the messages of this profile's game from now on; a change of game closes an open session.
    pub fn set_game(&self, profile: &GameProfile) {
        let wire = GameWire::of(profile);
        let mut probe_session = self
            .probe_session
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if probe_session.0 != wire {
            *probe_session = (wire, wire.session_build());
        }
        drop(probe_session);
        let _ = self.send(Command::SetGame(wire));
    }

    fn start(
        wire: GameWire,
        mut link: LocalLinkTransport,
        config: LauncherConfig,
        factory: TransportFactory,
    ) -> Result<Self, NetError> {
        link.host()
            .map_err(|error| NetError::Failed(error.to_string()))?;
        let link_port = link.local_port().ok_or(NetError::Unavailable)?;
        let status = Arc::new(Mutex::new(LinkStatus {
            link_port,
            ..LinkStatus::default()
        }));
        let (events_tx, events) = channel();
        let (commands, inbox) = channel();
        let bridge = wire.new_bridge(BridgeParts::new(
            link,
            Arc::clone(&factory),
            config.clone(),
            Arc::clone(&status),
            events_tx,
        ));
        let worker = thread::Builder::new()
            .name("rc-launcher-net".to_owned())
            .spawn(move || run_worker(bridge, wire, &inbox))
            .map_err(|error| NetError::Failed(error.to_string()))?;
        Ok(Self {
            commands: Mutex::new(Some(commands)),
            events: Mutex::new(events),
            status,
            factory,
            probe_session: Mutex::new((wire, config.build)),
            config,
            link_port,
            worker: Mutex::new(Some(worker)),
        })
    }

    pub fn link_port(&self) -> u16 {
        self.link_port
    }

    pub fn set_player_name(&self, name: &str) {
        let _ = self.send(Command::SetName(name.to_owned()));
    }

    pub fn set_rules(&self, rules: SessionRules) {
        let _ = self.send(Command::SetRules(rules));
    }

    pub fn remove_member(&self, slot: u8) {
        let _ = self.send(Command::RemoveMember(slot));
    }

    pub fn link_status(&self) -> LinkStatus {
        self.status
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub fn drain_events(&self) -> Vec<LauncherEvent> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .try_iter()
            .collect()
    }

    fn send(&self, command: Command) -> Result<(), NetError> {
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .ok_or(NetError::Unavailable)?
            .send(command)
            .map_err(|_| NetError::Unavailable)
    }
}

impl LauncherNet for LauncherCore {
    fn host(&self) -> Result<String, NetError> {
        let (reply, answer) = channel();
        self.send(Command::Host { reply })?;
        answer
            .recv_timeout(COMMAND_REPLY_TIMEOUT)
            .map_err(|_| NetError::Unavailable)?
    }

    fn join(&self, ticket: &str) -> Result<(), NetError> {
        let (reply, answer) = channel();
        self.send(Command::Join {
            ticket: ticket.to_owned(),
            reply,
        })?;
        answer
            .recv_timeout(COMMAND_REPLY_TIMEOUT)
            .map_err(|_| NetError::Unavailable)?
    }

    fn test_connection(&self, ticket: &str, seconds: u32) -> Result<ConnectionReport, NetError> {
        let (wire, build) = *self
            .probe_session
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let config = LauncherConfig {
            name: self.config.name.clone(),
            build,
        };
        wire.run_probe(&self.factory, &config, ticket, seconds)
    }

    fn leave(&self) {
        let _ = self.send(Command::Leave);
    }

    fn set_session_options(&self, options: SessionOptions) {
        let _ = self.send(Command::SetOptions(options));
    }

    fn set_player_settings(&self, settings: PlayerSettings) {
        let _ = self.send(Command::SetSettings(settings));
    }

    fn send_lobby_ready(&self, ready: LobbyReady) {
        let _ = self.send(Command::LobbyReady(ready));
    }

    fn send_lobby_body(&self, body: PartnerBody) {
        let _ = self.send(Command::LobbyBody(body));
    }

    fn send_lobby_character(&self, index: u8) {
        let _ = self.send(Command::LobbyCharacter(index));
    }

    fn set_rules(&self, rules: SessionRules) {
        LauncherCore::set_rules(self, rules);
    }

    fn start_lobby(&self, mode: u8) {
        let _ = self.send(Command::StartLobby(mode));
    }

    fn ask_game_to_quit(&self) {
        let _ = self.send(Command::QuitGame);
    }

    fn status(&self) -> NetStatus {
        let link = self.link_status();
        let state = match link.state {
            ConnectionState::Idle => NetState::Idle,
            ConnectionState::Hosting => NetState::Hosting {
                ticket: link.ticket.unwrap_or_default(),
            },
            ConnectionState::Joining => NetState::Joining,
            ConnectionState::Connected => NetState::Connected {
                peer_name: link
                    .partner()
                    .map(|partner| partner.name.clone())
                    .unwrap_or_default(),
            },
            ConnectionState::Rejected(reason) => {
                NetState::Failed(judge_rejection_text(reason).to_owned())
            }
            ConnectionState::Failed(message) => NetState::Failed(message),
        };
        NetStatus { state }
    }
}

impl Drop for LauncherCore {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl LauncherCore {
    pub fn shutdown(&self) {
        *self.commands.lock().unwrap_or_else(PoisonError::into_inner) = None;
        if let Some(worker) = self
            .worker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            let _ = worker.join();
        }
    }
}

fn run_worker(mut bridge: Box<dyn GameBridge>, mut wire: GameWire, inbox: &Receiver<Command>) {
    loop {
        match inbox.recv_timeout(WORKER_WAKE_INTERVAL) {
            Ok(Command::Host { reply }) => {
                let _ = reply.send(bridge.host());
            }
            Ok(Command::Join { ticket, reply }) => {
                let _ = reply.send(bridge.join(&ticket));
            }
            Ok(Command::Leave) => bridge.stop(),
            Ok(Command::SetOptions(options)) => bridge.set_options(options),
            Ok(Command::SetRules(rules)) => bridge.set_rules(rules),
            Ok(Command::SetSettings(settings)) => bridge.set_settings(settings),
            Ok(Command::SetName(name)) => bridge.set_name(&name),
            Ok(Command::LobbyReady(ready)) => bridge.set_lobby_ready(ready),
            Ok(Command::LobbyBody(body)) => bridge.set_lobby_body(body),
            Ok(Command::LobbyCharacter(index)) => bridge.set_lobby_character(index),
            Ok(Command::StartLobby(mode)) => bridge.start_lobby(mode),
            Ok(Command::RemoveMember(slot)) => bridge.remove_member(slot),
            Ok(Command::QuitGame) => bridge.quit_game(),
            Ok(Command::SetGame(wanted)) if wanted == wire => {}
            Ok(Command::SetGame(wanted)) => {
                let mut parts = bridge.into_parts();
                // A character index means nothing under another game's character list.
                parts.own_character = None;
                parts.config.build = wanted.session_build();
                parts.link.accept_only_builds(&wanted.hook_builds());
                bridge = wanted.new_bridge(parts);
                wire = wanted;
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                bridge.stop();
                return;
            }
        }
        bridge.pump(Instant::now());
    }
}
