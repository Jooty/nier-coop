use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use nc_wire::{STORY_MODE, SessionRules};

use crate::net::net_state::NetState;
use crate::net::player::Player;

pub const RULES_BEFORE_THE_WELCOME: SessionRules = SessionRules {
    mode: STORY_MODE,
    max_players: 2,
};

#[derive(Debug, Clone, PartialEq)]
pub struct NetShared<S, P, Y, X> {
    pub state: NetState,
    pub link_port: Option<u16>,
    pub local_slot: Option<u8>,
    pub rules: SessionRules,
    pub players: BTreeMap<u8, Player<P, Y>>,
    pub rtt: Option<Duration>,
    pub direct_path: Option<bool>,
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
            rules: RULES_BEFORE_THE_WELCOME,
            players: BTreeMap::new(),
            rtt: None,
            direct_path: None,
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
