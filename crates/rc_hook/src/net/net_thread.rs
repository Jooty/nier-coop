use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use rc_net::Transport;

use crate::net::local_snapshot::LocalSnapshot;
use crate::net::shared::{NetShared, SharedNet};
use crate::net::worker::Worker;

pub struct NetThread {
    snapshots: Sender<LocalSnapshot>,
    shared: SharedNet,
    wants_snapshots: Arc<AtomicBool>,
}

impl NetThread {
    pub fn spawn<T>(transport: T, link_port: u16) -> io::Result<Self>
    where
        T: Transport + 'static,
    {
        let (snapshots, snapshot_inbox) = channel();
        let shared: SharedNet = Arc::new(Mutex::new(NetShared::default()));
        let wants_snapshots = Arc::new(AtomicBool::new(false));
        let worker = Worker::new(
            transport,
            link_port,
            Arc::clone(&shared),
            Arc::clone(&wants_snapshots),
        );
        thread::Builder::new()
            .name("rc-net".to_owned())
            .spawn(move || worker.run(&snapshot_inbox))?;
        Ok(Self {
            snapshots,
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
