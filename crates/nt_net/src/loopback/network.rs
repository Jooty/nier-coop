use std::sync::{Arc, Mutex, PoisonError};

use crate::loopback::LoopbackTransport;
use crate::loopback::hub::Hub;
use crate::transport::PeerId;

#[derive(Clone, Default)]
pub struct LoopbackNetwork {
    hub: Arc<Mutex<Hub>>,
}

impl LoopbackNetwork {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn endpoint(&self) -> LoopbackTransport {
        let mut hub = self.hub.lock().unwrap_or_else(PoisonError::into_inner);
        hub.next_peer_number += 1;
        let mut id = [0u8; 32];
        id[..8].copy_from_slice(&hub.next_peer_number.to_le_bytes());
        let peer = PeerId(id);
        hub.inboxes.insert(peer, Vec::new());
        drop(hub);
        LoopbackTransport::new(peer, Arc::clone(&self.hub))
    }

    pub fn drop_all_messages(&self, dropping: bool) {
        let mut hub = self.hub.lock().unwrap_or_else(PoisonError::into_inner);
        hub.dropping_messages = dropping;
    }
}
