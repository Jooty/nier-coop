use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use rc_net::{LocalLinkTransport, Transport};
use rc_proto::{PlayerSettings, SessionOptions};

use crate::bridge::Bridge;
use crate::probe::run_probe;
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
    SetSettings(PlayerSettings),
    SetName(String),
}

pub struct LauncherCore {
    commands: Mutex<Option<Sender<Command>>>,
    events: Mutex<Receiver<LauncherEvent>>,
    status: Arc<Mutex<LinkStatus>>,
    factory: TransportFactory,
    config: LauncherConfig,
    link_port: u16,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl LauncherCore {
    pub fn new(config: LauncherConfig, factory: TransportFactory) -> Result<Self, NetError> {
        let mut link = LocalLinkTransport::launcher(Some(config.build));
        link.host()
            .map_err(|error| NetError::Failed(error.to_string()))?;
        let link_port = link.local_port().ok_or(NetError::Unavailable)?;
        let status = Arc::new(Mutex::new(LinkStatus {
            link_port,
            ..LinkStatus::default()
        }));
        let (events_tx, events) = channel();
        let (commands, inbox) = channel();
        let mut bridge = Bridge::new(
            link,
            Arc::clone(&factory),
            config.clone(),
            Arc::clone(&status),
            events_tx,
        );
        let worker = thread::Builder::new()
            .name("rc-launcher-net".to_owned())
            .spawn(move || run_worker(&mut bridge, &inbox))
            .map_err(|error| NetError::Failed(error.to_string()))?;
        Ok(Self {
            commands: Mutex::new(Some(commands)),
            events: Mutex::new(events),
            status,
            factory,
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
        run_probe(&self.factory, &self.config, ticket, seconds)
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

    fn status(&self) -> NetStatus {
        let link = self.link_status();
        let state = match link.state {
            ConnectionState::Idle => NetState::Idle,
            ConnectionState::Hosting => NetState::Hosting {
                ticket: link.ticket.unwrap_or_default(),
            },
            ConnectionState::Joining => NetState::Joining,
            ConnectionState::Connected => NetState::Connected {
                peer_name: link.peer_name.unwrap_or_default(),
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

fn run_worker(bridge: &mut Bridge, inbox: &Receiver<Command>) {
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
            Ok(Command::SetSettings(settings)) => bridge.set_settings(settings),
            Ok(Command::SetName(name)) => bridge.set_name(&name),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                bridge.stop();
                return;
            }
        }
        bridge.pump(Instant::now());
    }
}
