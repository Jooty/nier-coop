use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::net::net_state::NetState;
use crate::net::player::Player;
use crate::net::remote_player::RemotePlayer;

#[derive(Debug, Clone, PartialEq)]
pub struct NetShared<S, P, Y, X> {
    pub state: NetState,
    pub link_port: Option<u16>,
    pub local_slot: Option<u8>,
    pub players: BTreeMap<u8, Player<P, Y>>,
    /// Kept equal to `partner_name()` by the net worker until no reader names this field.
    pub peer_name: Option<String>,
    pub rtt: Option<Duration>,
    pub direct_path: Option<bool>,
    /// The single remote of a game that does not write `players` yet; `partner_state()` reads the map.
    pub remote: Option<RemotePlayer<P>>,
    pub local: Option<S>,
    pub quit_requested: bool,
    pub extras: X,
}

pub type SharedNet<S, P, Y, X> = Arc<Mutex<NetShared<S, P, Y, X>>>;

impl<S, P, Y, X: Default> Default for NetShared<S, P, Y, X> {
    fn default() -> Self {
        Self {
            state: NetState::default(),
            link_port: None,
            local_slot: None,
            players: BTreeMap::new(),
            peer_name: None,
            rtt: None,
            direct_path: None,
            remote: None,
            local: None,
            quit_requested: false,
            extras: X::default(),
        }
    }
}

impl<S, P, Y, X> Deref for NetShared<S, P, Y, X> {
    type Target = X;

    fn deref(&self) -> &X {
        &self.extras
    }
}

impl<S, P, Y, X> DerefMut for NetShared<S, P, Y, X> {
    fn deref_mut(&mut self) -> &mut X {
        &mut self.extras
    }
}
