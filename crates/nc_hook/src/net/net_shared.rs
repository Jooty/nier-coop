use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::net::net_state::NetState;
use crate::net::remote_player::RemotePlayer;

#[derive(Debug, Clone, PartialEq)]
pub struct NetShared<S, P, X> {
    pub state: NetState,
    pub link_port: Option<u16>,
    pub peer_name: Option<String>,
    pub rtt: Option<Duration>,
    pub direct_path: Option<bool>,
    pub remote: Option<RemotePlayer<P>>,
    pub local: Option<S>,
    pub local_slot: Option<u8>,
    pub quit_requested: bool,
    pub extras: X,
}

pub type SharedNet<S, P, X> = Arc<Mutex<NetShared<S, P, X>>>;

impl<S, P, X: Default> Default for NetShared<S, P, X> {
    fn default() -> Self {
        Self {
            state: NetState::default(),
            link_port: None,
            peer_name: None,
            rtt: None,
            direct_path: None,
            remote: None,
            local: None,
            local_slot: None,
            quit_requested: false,
            extras: X::default(),
        }
    }
}

impl<S, P, X> Deref for NetShared<S, P, X> {
    type Target = X;

    fn deref(&self) -> &X {
        &self.extras
    }
}

impl<S, P, X> DerefMut for NetShared<S, P, X> {
    fn deref_mut(&mut self) -> &mut X {
        &mut self.extras
    }
}
