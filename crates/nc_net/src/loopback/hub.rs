use std::collections::{HashMap, HashSet};

use crate::transport::{PeerId, TransportEvent};

#[derive(Default)]
pub(super) struct Hub {
    pub(super) next_peer_number: u64,
    pub(super) invite_counter: u64,
    pub(super) hosts: HashMap<String, PeerId>,
    pub(super) inboxes: HashMap<PeerId, Vec<TransportEvent>>,
    pub(super) links: HashSet<(PeerId, PeerId)>,
    pub(super) dropping_messages: bool,
}
