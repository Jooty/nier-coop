use std::collections::VecDeque;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Instant;

use nc_net::Transport;
use replicant_proto::{GameReadiness, Message, PlayerSettings, SessionOptions};

use crate::net::local_snapshot::LocalSnapshot;
use crate::net::shared::{NetShared, SharedNet};
use crate::net::worker::Worker;

pub struct NetThread {
    snapshots: Sender<LocalSnapshot>,
    relay_outbox: Sender<Message>,
    launcher_outbox: Sender<Message>,
    relay_inbox: Arc<Mutex<VecDeque<Message>>>,
    shared: SharedNet,
    wants_snapshots: Arc<AtomicBool>,
}

impl NetThread {
    pub fn spawn<T>(transport: T, link_port: u16) -> io::Result<Self>
    where
        T: Transport + 'static,
    {
        let (snapshots, snapshot_inbox) = channel();
        let (relay_outbox, relay_queue) = channel();
        let (launcher_outbox, launcher_queue) = channel();
        let relay_inbox = Arc::new(Mutex::new(VecDeque::new()));
        let shared: SharedNet = Arc::new(Mutex::new(NetShared::default()));
        let wants_snapshots = Arc::new(AtomicBool::new(false));
        let worker = Worker::new(
            transport,
            link_port,
            Arc::clone(&shared),
            Arc::clone(&wants_snapshots),
            Arc::clone(&relay_inbox),
        );
        thread::Builder::new()
            .name("rc-net".to_owned())
            .spawn(move || worker.run(&snapshot_inbox, &relay_queue, &launcher_queue))?;
        Ok(Self {
            snapshots,
            relay_outbox,
            launcher_outbox,
            relay_inbox,
            shared,
            wants_snapshots,
        })
    }

    pub fn publish(&self, snapshot: LocalSnapshot) {
        self.shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .local = Some(snapshot);
        let _ = self.snapshots.send(snapshot);
    }

    /// Queues a game message for the other player; dropped unless a peer is connected.
    pub fn send_relayed(&self, message: Message) {
        let _ = self.relay_outbox.send(message);
    }

    /// Removes and returns the waiting game messages `wanted` accepts, oldest first; others stay queued.
    pub fn take_relayed(&self, wanted: impl Fn(&Message) -> bool) -> Vec<Message> {
        let mut inbox = self
            .relay_inbox
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let (taken, kept): (VecDeque<Message>, VecDeque<Message>) =
            inbox.drain(..).partition(|message| wanted(message));
        *inbox = kept;
        taken.into()
    }

    pub fn take_launcher_settings(&self) -> Option<PlayerSettings> {
        self.shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .pending_settings
            .take()
    }

    pub fn tell_launcher_settings(&self, settings: PlayerSettings) {
        let _ = self.launcher_outbox.send(Message::Settings(settings));
    }

    pub fn tell_launcher_host_options(&self, options: SessionOptions) {
        self.shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .options = options;
        let _ = self.launcher_outbox.send(Message::SessionOptions(options));
    }

    pub fn report_readiness(&self, readiness: GameReadiness) {
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        shared.local_readiness = readiness;
        shared.local_readiness_at = Some(Instant::now());
    }

    pub fn take_quit_request(&self) -> bool {
        std::mem::take(
            &mut self
                .shared
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .quit_requested,
        )
    }

    pub fn wants_snapshots(&self) -> bool {
        self.wants_snapshots.load(Ordering::Relaxed)
    }

    pub fn view(&self) -> NetShared {
        self.shared
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}
