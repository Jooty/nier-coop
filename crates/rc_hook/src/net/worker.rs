use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, MutexGuard, PoisonError};
use std::time::Instant;

use rc_net::{Invite, NetError, Role, Session, SessionConfig, SessionEvent, Transport};
use rc_proto::BuildGate;

use crate::net::command::NetCommand;
use crate::net::local_snapshot::LocalSnapshot;
use crate::net::shared::{NetShared, NetState, RemotePlayer, SharedNet};
use crate::net::{LINK_STATS_INTERVAL, SEND_INTERVAL, WORKER_WAKE_INTERVAL};

pub(super) struct Worker<T: Transport, F: Fn() -> Result<T, NetError>> {
    make_transport: F,
    name: String,
    build: BuildGate,
    shared: SharedNet,
    wants_snapshots: Arc<AtomicBool>,
    session: Option<Session<T>>,
    next_tick: u32,
    latest_snapshot: Option<LocalSnapshot>,
    last_sent: Option<Instant>,
    last_link_update: Option<Instant>,
}

impl<T: Transport, F: Fn() -> Result<T, NetError>> Worker<T, F> {
    pub(super) fn new(
        make_transport: F,
        name: String,
        build: BuildGate,
        shared: SharedNet,
        wants_snapshots: Arc<AtomicBool>,
    ) -> Self {
        Self {
            make_transport,
            name,
            build,
            shared,
            wants_snapshots,
            session: None,
            next_tick: 0,
            latest_snapshot: None,
            last_sent: None,
            last_link_update: None,
        }
    }

    pub(super) fn run(
        mut self,
        commands: &Receiver<NetCommand>,
        snapshots: &Receiver<LocalSnapshot>,
    ) {
        loop {
            match commands.recv_timeout(WORKER_WAKE_INTERVAL) {
                Ok(command) => self.on_command(command),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.leave();
                    return;
                }
            }
            if let Some(snapshot) = snapshots.try_iter().last() {
                self.latest_snapshot = Some(snapshot);
            }
            self.pump(Instant::now());
        }
    }

    fn on_command(&mut self, command: NetCommand) {
        match command {
            NetCommand::Host => self.start(Role::Host, None),
            NetCommand::Join(ticket) => self.start(Role::Guest, Some(ticket)),
            NetCommand::Leave => {
                self.leave();
                self.shared().state = NetState::Idle;
            }
        }
    }

    fn start(&mut self, role: Role, ticket: Option<String>) {
        self.leave();
        self.shared().state = NetState::Starting;
        match self.open_session(role, ticket) {
            Ok(session) => self.session = Some(session),
            Err(error) => self.shared().state = NetState::Failed(error.to_string()),
        }
    }

    fn open_session(&self, role: Role, ticket: Option<String>) -> Result<Session<T>, NetError> {
        let transport = (self.make_transport)()?;
        let mut session = Session::new(
            transport,
            SessionConfig {
                role,
                name: self.name.clone(),
                build: self.build,
            },
        );
        match ticket {
            None => {
                let invite = session.host()?;
                let mut shared = self.shared();
                shared.ticket = Some(invite.as_str().to_owned());
                shared.state = NetState::Hosting;
            }
            Some(text) => {
                session.join(&Invite::new(text.trim().to_owned())?)?;
                self.shared().state = NetState::Joining;
            }
        }
        Ok(session)
    }

    fn leave(&mut self) {
        if let Some(mut session) = self.session.take() {
            session.close();
        }
        self.wants_snapshots.store(false, Ordering::Relaxed);
        self.latest_snapshot = None;
        let mut shared = self.shared();
        shared.state = NetState::Idle;
        shared.ticket = None;
        shared.peer_name = None;
        shared.rtt = None;
        shared.direct_path = None;
        shared.remote = None;
    }

    fn pump(&mut self, now: Instant) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let mut events = Vec::new();
        session.poll(now, &mut events);
        let is_host = session.local_slot() == Some(0);
        for event in events {
            self.on_event(event, is_host);
        }
        self.send_latest_snapshot(now);
        self.refresh_link_stats(now);
    }

    fn on_event(&mut self, event: SessionEvent, is_host: bool) {
        match event {
            SessionEvent::PeerJoined { name, .. } => {
                let mut shared = self.shared();
                shared.state = NetState::Connected;
                shared.peer_name = Some(name);
                drop(shared);
                self.wants_snapshots.store(true, Ordering::Relaxed);
            }
            SessionEvent::PeerState { state, .. } => {
                let mut shared = self.shared();
                let name = shared.peer_name.clone().unwrap_or_default();
                shared.remote = Some(RemotePlayer {
                    name,
                    slot: u8::from(is_host),
                    state,
                    received_at: Instant::now(),
                });
            }
            SessionEvent::PeerLeft { .. } => self.on_peer_left(is_host),
            SessionEvent::Rejected { reason } => {
                self.session = None;
                self.wants_snapshots.store(false, Ordering::Relaxed);
                let mut shared = self.shared();
                shared.state = NetState::Rejected(reason);
                shared.remote = None;
            }
            SessionEvent::PeerRefused { .. } => {}
        }
    }

    fn on_peer_left(&mut self, is_host: bool) {
        self.wants_snapshots.store(false, Ordering::Relaxed);
        if is_host {
            let mut shared = self.shared();
            shared.state = NetState::Hosting;
            shared.peer_name = None;
            shared.rtt = None;
            shared.direct_path = None;
            shared.remote = None;
        } else {
            self.leave();
            self.shared().state = NetState::Failed("the host left".to_owned());
        }
    }

    fn send_latest_snapshot(&mut self, now: Instant) {
        let due = self
            .last_sent
            .is_none_or(|sent| now.saturating_duration_since(sent) >= SEND_INTERVAL);
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if !due || session.ready_peers().is_empty() {
            return;
        }
        let Some(snapshot) = self.latest_snapshot.take() else {
            return;
        };
        self.next_tick = self.next_tick.wrapping_add(1);
        if session
            .send_state(&snapshot.to_player_state(self.next_tick))
            .is_ok()
        {
            self.last_sent = Some(now);
        }
    }

    fn refresh_link_stats(&mut self, now: Instant) {
        let due = self
            .last_link_update
            .is_none_or(|updated| now.saturating_duration_since(updated) >= LINK_STATS_INTERVAL);
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let Some(peer) = session.ready_peers().first().copied() else {
            return;
        };
        if !due {
            return;
        }
        self.last_link_update = Some(now);
        let rtt = session.rtt(peer);
        let direct = session
            .transport()
            .link_stats(peer)
            .map(|stats| stats.direct);
        let mut shared = self.shared();
        shared.rtt = rtt;
        shared.direct_path = direct;
    }

    fn shared(&self) -> MutexGuard<'_, NetShared> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
