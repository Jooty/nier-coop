use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, PoisonError};
use std::time::{Duration, Instant};

use iroh::endpoint::presets;
use iroh::{Endpoint, EndpointAddr, RelayMode, Watcher};
use tokio::runtime::{Builder, Runtime};

use crate::iroh_adapter::invite_codec::{decode_invite, encode_invite};
use crate::iroh_adapter::link::{PeerTable, start_link};
use crate::iroh_adapter::own_address::{relay_label, report_own_address};
use crate::iroh_adapter::path_report::report_paths;
use crate::iroh_adapter::{
    ALPN, BYE_CODE, CLOSE_WAIT_SECONDS, DIRECT_ADDRESS_POLL_INTERVAL, DIRECT_ADDRESS_POLLS,
    IrohConfig, RELAY_CHOICE_WAIT_SECONDS, WORKER_THREADS,
};
use crate::progress::{LinkProgress, ProgressSink};
use crate::transport::{
    Channel, Invite, LinkStats, MAX_PAYLOAD_BYTES, NetError, PeerId, Transport, TransportEvent,
};

pub struct IrohTransport {
    runtime: Runtime,
    endpoint: Endpoint,
    config: IrohConfig,
    local: PeerId,
    peers: PeerTable,
    events_tx: Sender<TransportEvent>,
    events_rx: Receiver<TransportEvent>,
    bound_at: Instant,
    progress: Option<ProgressSink>,
}

impl IrohTransport {
    pub fn new(config: IrohConfig) -> Result<Self, NetError> {
        let runtime = Builder::new_multi_thread()
            .worker_threads(WORKER_THREADS)
            .thread_name("rc-net-iroh")
            .enable_all()
            .build()
            .map_err(transport_error)?;
        let endpoint = runtime.block_on(bind_endpoint(config))?;
        let bound_at = Instant::now();
        let local = PeerId(*endpoint.id().as_bytes());
        let peers = PeerTable::default();
        let (events_tx, events_rx) = channel();
        runtime.spawn(accept_loop(
            endpoint.clone(),
            Arc::clone(&peers),
            events_tx.clone(),
        ));
        Ok(Self {
            runtime,
            endpoint,
            config,
            local,
            peers,
            events_tx,
            events_rx,
            bound_at,
            progress: None,
        })
    }
}

impl Transport for IrohTransport {
    fn local_peer(&self) -> PeerId {
        self.local
    }

    fn host(&mut self) -> Result<Invite, NetError> {
        let endpoint = self.endpoint.clone();
        let use_relays = self.config.use_relays_and_lookup;
        self.runtime.block_on(async move {
            if use_relays {
                // The ticket needs the relay's name only: the relay's own answer can come after it.
                let wait = Duration::from_secs(RELAY_CHOICE_WAIT_SECONDS);
                let _ = tokio::time::timeout(wait, wait_for_relay_choice(&endpoint)).await;
            } else {
                wait_for_direct_address(&endpoint).await;
            }
        });
        encode_invite(&self.endpoint.addr())
    }

    fn join(&mut self, invite: &Invite) -> Result<(), NetError> {
        let addr = decode_invite(invite)?;
        if let Some(sink) = &self.progress {
            sink.send(ticket_read(&addr));
        }
        let endpoint = self.endpoint.clone();
        let peers = Arc::clone(&self.peers);
        let events = self.events_tx.clone();
        let progress = self.progress.clone();
        self.runtime.spawn(async move {
            let dialed_at = Instant::now();
            match endpoint.connect(addr, ALPN).await {
                Ok(connection) => {
                    if let Some(sink) = progress {
                        sink.send(LinkProgress::HostAnswered {
                            after: dialed_at.elapsed(),
                        });
                        tokio::spawn(report_paths(connection.clone(), sink));
                    }
                    start_link(connection, peers, events);
                }
                Err(_) => {
                    if let Some(sink) = progress {
                        sink.send(LinkProgress::DialFailed);
                    }
                }
            }
        });
        Ok(())
    }

    fn send(&mut self, to: PeerId, channel: Channel, payload: &[u8]) -> Result<(), NetError> {
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(NetError::PayloadTooLarge {
                len: payload.len(),
                limit: MAX_PAYLOAD_BYTES,
            });
        }
        let peers = self.peers.lock().unwrap_or_else(PoisonError::into_inner);
        let link = peers.get(&to).ok_or(NetError::NotConnected)?;
        match channel {
            Channel::UnreliableLatest => link
                .connection
                .send_datagram(payload.to_vec().into())
                .map_err(transport_error),
            Channel::ReliableOrdered => link
                .reliable_outbox
                .send(payload.to_vec())
                .map_err(|_| NetError::NotConnected),
        }
    }

    fn poll(&mut self, out: &mut Vec<TransportEvent>) {
        out.extend(self.events_rx.try_iter());
    }

    fn link_stats(&self, peer: PeerId) -> Option<LinkStats> {
        let peers = self.peers.lock().unwrap_or_else(PoisonError::into_inner);
        let connection = &peers.get(&peer)?.connection;
        let paths = connection.paths();
        let selected = paths.iter().find(|path| path.is_selected())?;
        let stats = selected.stats();
        Some(LinkStats {
            rtt: Some(stats.rtt),
            direct: selected.is_ip(),
            lost_packets: stats.lost_packets,
        })
    }

    fn disconnect(&mut self, peer: PeerId) {
        let peers = self.peers.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(link) = peers.get(&peer) {
            link.connection.close(BYE_CODE.into(), b"bye");
        }
    }

    fn report_progress(&mut self, sink: ProgressSink) {
        sink.send_at(self.bound_at, bound(&self.endpoint.bound_sockets()));
        sink.send_at(
            self.bound_at,
            LinkProgress::OwnId(self.endpoint.id().fmt_short().to_string()),
        );
        self.runtime
            .spawn(report_own_address(self.endpoint.clone(), sink.clone()));
        self.progress = Some(sink);
    }
}

impl Drop for IrohTransport {
    fn drop(&mut self) {
        let wait = Duration::from_secs(CLOSE_WAIT_SECONDS);
        let endpoint = self.endpoint.clone();
        let _ = self
            .runtime
            .block_on(async move { tokio::time::timeout(wait, endpoint.close()).await });
    }
}

async fn bind_endpoint(config: IrohConfig) -> Result<Endpoint, NetError> {
    let builder = if config.use_relays_and_lookup {
        Endpoint::builder(presets::N0)
    } else {
        Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled)
    };
    builder
        .alpns(vec![ALPN.to_vec()])
        .bind()
        .await
        .map_err(transport_error)
}

async fn wait_for_relay_choice(endpoint: &Endpoint) {
    let mut addr = endpoint.watch_addr();
    while addr.get().relay_urls().next().is_none() {
        if addr.updated().await.is_err() {
            return;
        }
    }
}

async fn wait_for_direct_address(endpoint: &Endpoint) {
    for _ in 0..DIRECT_ADDRESS_POLLS {
        if endpoint.addr().ip_addrs().next().is_some() {
            return;
        }
        tokio::time::sleep(DIRECT_ADDRESS_POLL_INTERVAL).await;
    }
}

async fn accept_loop(endpoint: Endpoint, peers: PeerTable, events: Sender<TransportEvent>) {
    while let Some(incoming) = endpoint.accept().await {
        let Ok(accepting) = incoming.accept() else {
            continue;
        };
        let peers = Arc::clone(&peers);
        let events = events.clone();
        tokio::spawn(async move {
            if let Ok(connection) = accepting.await {
                start_link(connection, peers, events);
            }
        });
    }
}

fn bound(sockets: &[SocketAddr]) -> LinkProgress {
    let port_of = |ipv6: bool| {
        sockets
            .iter()
            .find(|socket| socket.is_ipv6() == ipv6)
            .map(SocketAddr::port)
    };
    LinkProgress::Bound {
        ipv4_port: port_of(false),
        ipv6_port: port_of(true),
    }
}

fn ticket_read(addr: &EndpointAddr) -> LinkProgress {
    LinkProgress::TicketRead {
        host_id: addr.id.fmt_short().to_string(),
        relay: addr.relay_urls().next().map(relay_label),
        direct_addresses: addr.ip_addrs().count(),
    }
}

fn transport_error(error: impl std::fmt::Display) -> NetError {
    NetError::Transport(error.to_string())
}
