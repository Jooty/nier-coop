use automata_game::character::Character;

use crate::net::link_state::NetShared;
use crate::net::load_signal::HeardLoadSignal;

/// What this game has heard from one other player since they joined.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PartnerWatch {
    pub character: Option<Character>,
    /// The other game's hello or a state has arrived since the join.
    pub heard: bool,
    pub load: Option<HeardLoadSignal>,
}

impl PartnerWatch {
    pub fn state_age_ms(&self, now_ms: u64) -> Option<u64> {
        self.load
            .map(|heard| now_ms.saturating_sub(heard.heard_at_ms))
    }
}

pub trait NetSharedExt {
    /// The partner's watch, or an empty one while no partner is in the session.
    fn partner_watch(&self) -> PartnerWatch;

    /// The watch of the player in one slot, or an empty one while the slot holds nobody.
    fn player_watch(&self, slot: u8) -> PartnerWatch;
}

impl NetSharedExt for NetShared {
    fn partner_watch(&self) -> PartnerWatch {
        self.partner()
            .map_or_else(PartnerWatch::default, |partner| partner.watch)
    }

    fn player_watch(&self, slot: u8) -> PartnerWatch {
        self.player(slot)
            .map_or_else(PartnerWatch::default, |player| player.watch)
    }
}
