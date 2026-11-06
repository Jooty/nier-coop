use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

use rc_net::{NetError, Transport};
use rc_proto::BuildGate;

use crate::net::command::NetCommand;
use crate::net::local_snapshot::LocalSnapshot;
use crate::net::shared::{NetShared, SharedNet};
use crate::net::worker::Worker;

pub struct NetThread {
    commands: Sender<NetCommand>,
    snapshots: Sender<LocalSnapshot>,
    shared: SharedNet,
    wants_snapshots: Arc<AtomicBool>,
}

impl NetThread {
    pub fn spawn<T, F>(make_transport: F, name: String, build: BuildGate) -> io::Result<Self>
    where
        T: Transport + 'static,
        F: Fn() -> Result<T, NetError> + Send + 'static,
    {
        let (commands, command_inbox) = channel();
        let (snapshots, snapshot_inbox) = channel();
        let shared: SharedNet = Arc::new(Mutex::new(NetShared::default()));
        let wants_snapshots = Arc::new(AtomicBool::new(false));
        let worker = Worker::new(
            make_transport,
            name,
            build,
            Arc::clone(&shared),
            Arc::clone(&wants_snapshots),
        );
        thread::Builder::new()
            .name("rc-net".to_owned())
            .spawn(move || worker.run(&command_inbox, &snapshot_inbox))?;
        Ok(Self {
            commands,
            snapshots,
            shared,
            wants_snapshots,
        })
    }

    pub fn send_command(&self, command: NetCommand) {
        let _ = self.commands.send(command);
    }

    pub fn publish(&self, snapshot: LocalSnapshot) {
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
