use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, PoisonError};

use iroh::endpoint::{Connection, ConnectionError};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::iroh_adapter::PROTOCOL_ERROR_CODE;
use crate::transport::{Channel, DisconnectReason, MAX_PAYLOAD_BYTES, PeerId, TransportEvent};

pub(super) struct PeerLink {
    pub(super) connection: Connection,
    pub(super) reliable_outbox: UnboundedSender<Vec<u8>>,
}

pub(super) type PeerTable = Arc<Mutex<HashMap<PeerId, PeerLink>>>;

pub(super) fn peer_of(connection: &Connection) -> PeerId {
    PeerId(*connection.remote_id().as_bytes())
}

pub(super) fn start_link(connection: Connection, peers: PeerTable, events: Sender<TransportEvent>) {
    let peer = peer_of(&connection);
    let (reliable_outbox, reliable_inbox) = unbounded_channel::<Vec<u8>>();
    peers.lock().unwrap_or_else(PoisonError::into_inner).insert(
        peer,
        PeerLink {
            connection: connection.clone(),
            reliable_outbox,
        },
    );
    let _ = events.send(TransportEvent::PeerConnected(peer));
    tokio::spawn(write_reliable(connection.clone(), reliable_inbox));
    tokio::spawn(read_reliable(connection.clone(), peer, events.clone()));
    tokio::spawn(read_datagrams(connection.clone(), peer, events.clone()));
    tokio::spawn(report_close(connection, peer, peers, events));
}

async fn write_reliable(connection: Connection, mut inbox: UnboundedReceiver<Vec<u8>>) {
    let Ok(mut stream) = connection.open_uni().await else {
        return;
    };
    while let Some(payload) = inbox.recv().await {
        let Ok(len) = u16::try_from(payload.len()) else {
            continue;
        };
        if stream.write_all(&len.to_be_bytes()).await.is_err()
            || stream.write_all(&payload).await.is_err()
        {
            return;
        }
    }
    let _ = stream.finish();
}

async fn read_reliable(connection: Connection, peer: PeerId, events: Sender<TransportEvent>) {
    let Ok(mut stream) = connection.accept_uni().await else {
        return;
    };
    loop {
        let mut len_bytes = [0u8; 2];
        if stream.read_exact(&mut len_bytes).await.is_err() {
            return;
        }
        let len = usize::from(u16::from_be_bytes(len_bytes));
        if len > MAX_PAYLOAD_BYTES {
            connection.close(PROTOCOL_ERROR_CODE.into(), b"oversized message");
            return;
        }
        let mut payload = vec![0u8; len];
        if stream.read_exact(&mut payload).await.is_err() {
            return;
        }
        let event = TransportEvent::Message {
            from: peer,
            channel: Channel::ReliableOrdered,
            payload,
        };
        if events.send(event).is_err() {
            return;
        }
    }
}

async fn read_datagrams(connection: Connection, peer: PeerId, events: Sender<TransportEvent>) {
    while let Ok(datagram) = connection.read_datagram().await {
        if datagram.len() > MAX_PAYLOAD_BYTES {
            continue;
        }
        let event = TransportEvent::Message {
            from: peer,
            channel: Channel::UnreliableLatest,
            payload: datagram.to_vec(),
        };
        if events.send(event).is_err() {
            return;
        }
    }
}

async fn report_close(
    connection: Connection,
    peer: PeerId,
    peers: PeerTable,
    events: Sender<TransportEvent>,
) {
    let error = connection.closed().await;
    let reason = match error {
        ConnectionError::TimedOut => DisconnectReason::TimedOut,
        ConnectionError::ApplicationClosed(_) | ConnectionError::LocallyClosed => {
            DisconnectReason::Closed
        }
        _ => DisconnectReason::Failed,
    };
    peers
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&peer);
    let _ = events.send(TransportEvent::PeerDisconnected(peer, reason));
}
