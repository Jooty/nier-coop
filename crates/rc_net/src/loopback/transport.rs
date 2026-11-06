use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use crate::loopback::hub::Hub;
use crate::transport::{
    Channel, DisconnectReason, Invite, LinkStats, MAX_PAYLOAD_BYTES, NetError, PeerId, Transport,
    TransportEvent,
};

const LOOPBACK_RTT: Duration = Duration::from_millis(1);
const INVITE_PREFIX: &str = "loopback-";

pub struct LoopbackTransport {
    id: PeerId,
    hub: Arc<Mutex<Hub>>,
}

impl LoopbackTransport {
    pub(super) fn new(id: PeerId, hub: Arc<Mutex<Hub>>) -> Self {
        Self { id, hub }
    }

    fn hub(&self) -> MutexGuard<'_, Hub> {
        self.hub.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Transport for LoopbackTransport {
    fn local_peer(&self) -> PeerId {
        self.id
    }

    fn host(&mut self) -> Result<Invite, NetError> {
        let mut hub = self.hub();
        hub.invite_counter += 1;
        let text = format!("{INVITE_PREFIX}{}", hub.invite_counter);
        hub.hosts.insert(text.clone(), self.id);
        drop(hub);
        Invite::new(text)
    }

    fn join(&mut self, invite: &Invite) -> Result<(), NetError> {
        let mut hub = self.hub();
        let host = *hub
            .hosts
            .get(invite.as_str())
            .ok_or(NetError::InvalidInvite)?;
        hub.links.insert((self.id, host));
        hub.links.insert((host, self.id));
        if let Some(inbox) = hub.inboxes.get_mut(&host) {
            inbox.push(TransportEvent::PeerConnected(self.id));
        }
        if let Some(inbox) = hub.inboxes.get_mut(&self.id) {
            inbox.push(TransportEvent::PeerConnected(host));
        }
        Ok(())
    }

    fn send(&mut self, to: PeerId, channel: Channel, payload: &[u8]) -> Result<(), NetError> {
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(NetError::PayloadTooLarge {
                len: payload.len(),
                limit: MAX_PAYLOAD_BYTES,
            });
        }
        let mut hub = self.hub();
        if !hub.links.contains(&(self.id, to)) {
            return Err(NetError::NotConnected);
        }
        if hub.dropping_messages {
            return Ok(());
        }
        if let Some(inbox) = hub.inboxes.get_mut(&to) {
            inbox.push(TransportEvent::Message {
                from: self.id,
                channel,
                payload: payload.to_vec(),
            });
        }
        Ok(())
    }

    fn poll(&mut self, out: &mut Vec<TransportEvent>) {
        let mut hub = self.hub();
        if let Some(inbox) = hub.inboxes.get_mut(&self.id) {
            out.append(inbox);
        }
    }

    fn link_stats(&self, peer: PeerId) -> Option<LinkStats> {
        self.hub()
            .links
            .contains(&(self.id, peer))
            .then_some(LinkStats {
                rtt: Some(LOOPBACK_RTT),
                direct: true,
                lost_packets: 0,
            })
    }

    fn disconnect(&mut self, peer: PeerId) {
        let mut hub = self.hub();
        if !hub.links.remove(&(self.id, peer)) {
            return;
        }
        hub.links.remove(&(peer, self.id));
        if let Some(inbox) = hub.inboxes.get_mut(&peer) {
            inbox.push(TransportEvent::PeerDisconnected(
                self.id,
                DisconnectReason::Closed,
            ));
        }
        if let Some(inbox) = hub.inboxes.get_mut(&self.id) {
            inbox.push(TransportEvent::PeerDisconnected(
                peer,
                DisconnectReason::Closed,
            ));
        }
    }
}
