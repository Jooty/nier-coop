use std::io::{Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use nt_wire::BuildGate;

use crate::local_link::frame::{
    ACK_BAD_HELLO, ACK_BUILD_MISMATCH, ACK_BUSY, ACK_OK, HANDSHAKE_TIMEOUT, WRITE_TIMEOUT,
    read_frame, read_hello, write_frame, write_hello,
};
use crate::local_link::{LINK_ADDRESS, LINK_GAME_PEER, LINK_INVITE_PREFIX, LINK_LAUNCHER_PEER};
use crate::transport::{
    Channel, DisconnectReason, Invite, LinkStats, MAX_PAYLOAD_BYTES, NetError, PeerId, Transport,
    TransportEvent,
};

const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(20);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
const RECONNECT_DELAY: Duration = Duration::from_millis(500);
const STOP_CHECK_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Copy)]
enum Side {
    Launcher,
    Game { build: BuildGate },
}

#[derive(Default)]
struct Attached {
    writer: Option<TcpStream>,
    generation: u64,
    client_build: Option<BuildGate>,
}

struct Shared {
    attached: Mutex<Attached>,
    accepted_builds: Mutex<Vec<BuildGate>>,
    stop: AtomicBool,
    next_generation: AtomicU64,
}

pub struct LocalLinkTransport {
    side: Side,
    shared: Arc<Shared>,
    events_tx: Sender<TransportEvent>,
    events_rx: Receiver<TransportEvent>,
    port: Option<u16>,
}

impl LocalLinkTransport {
    pub fn launcher(expected_build: Option<BuildGate>) -> Self {
        let link = Self::new(Side::Launcher);
        link.accept_only_builds(expected_build.as_slice());
        link
    }

    /// The builds a game may attach with from now on; an empty list lets any build attach.
    pub fn accept_only_builds(&self, builds: &[BuildGate]) {
        *lock(&self.shared.accepted_builds) = builds.to_vec();
        let attached = lock(&self.shared.attached);
        if let (Some(writer), Some(build)) = (attached.writer.as_ref(), attached.client_build)
            && !builds.is_empty()
            && !builds.contains(&build)
        {
            let _ = writer.shutdown(Shutdown::Both);
        }
    }

    pub fn game(build: BuildGate) -> Self {
        Self::new(Side::Game { build })
    }

    pub fn local_port(&self) -> Option<u16> {
        self.port
    }

    pub fn client_build(&self) -> Option<BuildGate> {
        lock(&self.shared.attached).client_build
    }

    pub fn is_attached(&self) -> bool {
        lock(&self.shared.attached).writer.is_some()
    }

    fn new(side: Side) -> Self {
        let (events_tx, events_rx) = channel();
        Self {
            side,
            shared: Arc::new(Shared {
                attached: Mutex::new(Attached::default()),
                accepted_builds: Mutex::new(Vec::new()),
                stop: AtomicBool::new(false),
                next_generation: AtomicU64::new(1),
            }),
            events_tx,
            events_rx,
            port: None,
        }
    }

    fn remote(&self) -> PeerId {
        match self.side {
            Side::Launcher => LINK_GAME_PEER,
            Side::Game { .. } => LINK_LAUNCHER_PEER,
        }
    }
}

impl Transport for LocalLinkTransport {
    fn local_peer(&self) -> PeerId {
        match self.side {
            Side::Launcher => LINK_LAUNCHER_PEER,
            Side::Game { .. } => LINK_GAME_PEER,
        }
    }

    fn host(&mut self) -> Result<Invite, NetError> {
        let Side::Launcher = self.side else {
            return Err(NetError::Transport(
                "only the launcher side listens".to_owned(),
            ));
        };
        let listener = TcpListener::bind((LINK_ADDRESS, 0)).map_err(io_error)?;
        listener.set_nonblocking(true).map_err(io_error)?;
        let port = listener.local_addr().map_err(io_error)?.port();
        self.port = Some(port);
        let shared = Arc::clone(&self.shared);
        let events = self.events_tx.clone();
        thread::Builder::new()
            .name("rc-link-accept".to_owned())
            .spawn(move || accept_loop(&listener, &shared, &events))
            .map_err(io_error)?;
        Invite::new(format!("{LINK_INVITE_PREFIX}{port}"))
    }

    fn join(&mut self, invite: &Invite) -> Result<(), NetError> {
        let Side::Game { build } = self.side else {
            return Err(NetError::Transport("only the game side dials".to_owned()));
        };
        let port = invite
            .as_str()
            .strip_prefix(LINK_INVITE_PREFIX)
            .and_then(|text| text.parse::<u16>().ok())
            .ok_or(NetError::InvalidInvite)?;
        let shared = Arc::clone(&self.shared);
        let events = self.events_tx.clone();
        thread::Builder::new()
            .name("rc-link-dial".to_owned())
            .spawn(move || dial_loop(port, build, &shared, &events))
            .map_err(io_error)?;
        Ok(())
    }

    fn send(&mut self, to: PeerId, channel: Channel, payload: &[u8]) -> Result<(), NetError> {
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(NetError::PayloadTooLarge {
                len: payload.len(),
                limit: MAX_PAYLOAD_BYTES,
            });
        }
        if to != self.remote() {
            return Err(NetError::NotConnected);
        }
        let attached = lock(&self.shared.attached);
        let Some(mut writer) = attached.writer.as_ref() else {
            return Err(NetError::NotConnected);
        };
        if write_frame(&mut writer, channel, payload).is_err() {
            let _ = writer.shutdown(Shutdown::Both);
            return Err(NetError::NotConnected);
        }
        Ok(())
    }

    fn poll(&mut self, out: &mut Vec<TransportEvent>) {
        out.extend(self.events_rx.try_iter());
    }

    fn link_stats(&self, peer: PeerId) -> Option<LinkStats> {
        (peer == self.remote() && self.is_attached()).then_some(LinkStats {
            rtt: Some(Duration::ZERO),
            direct: true,
            lost_packets: 0,
        })
    }

    fn disconnect(&mut self, peer: PeerId) {
        if peer != self.remote() {
            return;
        }
        if let Some(writer) = lock(&self.shared.attached).writer.as_ref() {
            let _ = writer.shutdown(Shutdown::Both);
        }
    }
}

impl Drop for LocalLinkTransport {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
        if let Some(writer) = lock(&self.shared.attached).writer.as_ref() {
            let _ = writer.shutdown(Shutdown::Both);
        }
    }
}

fn accept_loop(listener: &TcpListener, shared: &Arc<Shared>, events: &Sender<TransportEvent>) {
    while !shared.stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _)) => {
                let shared = Arc::clone(shared);
                let events = events.clone();
                let _ = thread::Builder::new()
                    .name("rc-link-serve".to_owned())
                    .spawn(move || serve(stream, &shared, &events));
            }
            Err(_) => thread::sleep(ACCEPT_POLL_INTERVAL),
        }
    }
}

fn serve(mut stream: TcpStream, shared: &Arc<Shared>, events: &Sender<TransportEvent>) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT));
    let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
    let Ok(hello) = read_hello(&mut stream) else {
        return;
    };
    let Some(build) = hello else {
        let _ = stream.write_all(&[ACK_BAD_HELLO]);
        return;
    };
    if !build_is_accepted(shared, build) {
        let _ = stream.write_all(&[ACK_BUILD_MISMATCH]);
        return;
    }
    let generation = shared.next_generation.fetch_add(1, Ordering::Relaxed);
    {
        let mut attached = lock(&shared.attached);
        if attached.writer.is_some() {
            drop(attached);
            let _ = stream.write_all(&[ACK_BUSY]);
            return;
        }
        let Ok(writer) = stream.try_clone() else {
            return;
        };
        if stream.write_all(&[ACK_OK]).is_err() {
            return;
        }
        attached.writer = Some(writer);
        attached.generation = generation;
        attached.client_build = Some(build);
    }
    let _ = events.send(TransportEvent::PeerConnected(LINK_GAME_PEER));
    read_until_closed(stream, LINK_GAME_PEER, generation, shared, events);
}

fn build_is_accepted(shared: &Shared, build: BuildGate) -> bool {
    let accepted = lock(&shared.accepted_builds);
    accepted.is_empty() || accepted.contains(&build)
}

fn dial_loop(port: u16, build: BuildGate, shared: &Arc<Shared>, events: &Sender<TransportEvent>) {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    while !shared.stop.load(Ordering::Relaxed) {
        if let Some(stream) = dial_once(address, build) {
            attach_dialed(stream, shared, events);
        }
        sleep_unless_stopped(RECONNECT_DELAY, shared);
    }
}

fn dial_once(address: SocketAddr, build: BuildGate) -> Option<TcpStream> {
    let mut stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).ok()?;
    stream.set_nodelay(true).ok()?;
    stream.set_read_timeout(Some(HANDSHAKE_TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(WRITE_TIMEOUT)).ok()?;
    write_hello(&mut stream, build).ok()?;
    let mut ack = [0u8; 1];
    stream.read_exact(&mut ack).ok()?;
    (ack[0] == ACK_OK).then_some(stream)
}

fn attach_dialed(stream: TcpStream, shared: &Arc<Shared>, events: &Sender<TransportEvent>) {
    let Ok(writer) = stream.try_clone() else {
        return;
    };
    let generation = shared.next_generation.fetch_add(1, Ordering::Relaxed);
    {
        let mut attached = lock(&shared.attached);
        attached.writer = Some(writer);
        attached.generation = generation;
    }
    let _ = events.send(TransportEvent::PeerConnected(LINK_LAUNCHER_PEER));
    read_until_closed(stream, LINK_LAUNCHER_PEER, generation, shared, events);
}

fn read_until_closed(
    mut stream: TcpStream,
    peer: PeerId,
    generation: u64,
    shared: &Arc<Shared>,
    events: &Sender<TransportEvent>,
) {
    let _ = stream.set_read_timeout(None);
    while let Ok((channel, payload)) = read_frame(&mut stream) {
        let event = TransportEvent::Message {
            from: peer,
            channel,
            payload,
        };
        if events.send(event).is_err() {
            break;
        }
    }
    {
        let mut attached = lock(&shared.attached);
        if attached.generation == generation {
            attached.writer = None;
        }
    }
    let _ = stream.shutdown(Shutdown::Both);
    let _ = events.send(TransportEvent::PeerDisconnected(
        peer,
        DisconnectReason::Closed,
    ));
}

fn sleep_unless_stopped(total: Duration, shared: &Shared) {
    let mut slept = Duration::ZERO;
    while slept < total && !shared.stop.load(Ordering::Relaxed) {
        thread::sleep(STOP_CHECK_INTERVAL);
        slept += STOP_CHECK_INTERVAL;
    }
}

fn lock<T>(guarded: &Mutex<T>) -> MutexGuard<'_, T> {
    guarded.lock().unwrap_or_else(PoisonError::into_inner)
}

fn io_error(error: impl std::fmt::Display) -> NetError {
    NetError::Transport(error.to_string())
}
