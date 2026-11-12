use crate::net::net_shared::NetShared;
use crate::net::player::Player;
use crate::net::remote_player::RemotePlayer;

impl<S, P, Y, X> NetShared<S, P, Y, X> {
    /// The other player of the session; a session holds at most one.
    pub fn partner(&self) -> Option<&Player<P, Y>> {
        self.players.values().next()
    }

    pub fn partner_state(&self) -> Option<&RemotePlayer<P>> {
        self.partner()?.latest.as_ref()
    }

    pub fn partner_name(&self) -> Option<&str> {
        self.partner().map(|partner| partner.name.as_str())
    }

    pub fn others(&self) -> impl Iterator<Item = &Player<P, Y>> {
        self.players.values()
    }

    pub fn player(&self, slot: u8) -> Option<&Player<P, Y>> {
        self.players.get(&slot)
    }
}
