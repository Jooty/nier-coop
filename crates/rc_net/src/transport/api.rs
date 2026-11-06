use crate::transport::{Channel, Invite, LinkStats, NetError, PeerId, TransportEvent};

pub trait Transport: Send {
    fn local_peer(&self) -> PeerId;
    fn host(&mut self) -> Result<Invite, NetError>;
    fn join(&mut self, invite: &Invite) -> Result<(), NetError>;
    fn send(&mut self, to: PeerId, channel: Channel, payload: &[u8]) -> Result<(), NetError>;
    fn poll(&mut self, out: &mut Vec<TransportEvent>);
    fn link_stats(&self, peer: PeerId) -> Option<LinkStats>;
    fn disconnect(&mut self, peer: PeerId);
}
