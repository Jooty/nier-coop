use std::fmt::Debug;
use std::sync::{MutexGuard, PoisonError};

use nc_net::Channel;
use nc_wire::GameMessage;

use crate::net::net_shared::{NetShared, SharedNet};
use crate::relay::RelayInbox;

pub type NetSharedOf<G> =
    NetShared<<G as NetGame>::Snapshot, <G as NetGame>::PeerState, <G as NetGame>::Extras>;
pub type SharedNetOf<G> =
    SharedNet<<G as NetGame>::Snapshot, <G as NetGame>::PeerState, <G as NetGame>::Extras>;

pub trait NetGame: Send + Sized + 'static {
    type Message: GameMessage + Send + 'static;
    type Snapshot: Copy + Debug + PartialEq + Send + 'static;
    type PeerState: Clone + Debug + PartialEq + Send + 'static;
    type Extras: Clone + Debug + Default + PartialEq + Send + 'static;

    fn kind_name(message: &Self::Message) -> &'static str;

    fn peer_in_play(extras: &Self::Extras) -> bool;

    fn snapshot_message(&mut self, snapshot: &Self::Snapshot, tick: u32) -> Self::Message;

    fn on_message(&mut self, link: &LinkAccess<'_, Self>, message: Self::Message);

    fn on_connected(&mut self, extras: &mut Self::Extras, peer_name: &str);

    fn on_peer_cleared(&mut self, extras: &mut Self::Extras);

    fn on_launcher_linked(&mut self);

    fn on_idle(
        &mut self,
        link: &LinkAccess<'_, Self>,
        send: &mut dyn FnMut(&Self::Message, Channel) -> bool,
    );

    fn announce(&self, text: &str);
}

pub struct LinkAccess<'a, G: NetGame> {
    shared: &'a SharedNetOf<G>,
    relay_inbox: &'a RelayInbox<G::Message>,
}

impl<'a, G: NetGame> LinkAccess<'a, G> {
    pub(crate) fn new(shared: &'a SharedNetOf<G>, relay_inbox: &'a RelayInbox<G::Message>) -> Self {
        Self {
            shared,
            relay_inbox,
        }
    }

    pub fn shared(&self) -> MutexGuard<'a, NetSharedOf<G>> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn push_relayed(&self, message: G::Message) {
        self.relay_inbox.push(message);
    }

    pub fn clear_relayed(&self) {
        self.relay_inbox.clear();
    }
}
