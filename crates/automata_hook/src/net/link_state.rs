use automata_game::character::Character;
use automata_game::partner::{BodySnapshot, RemoteInput};
use automata_proto::PlayerState;

use crate::net::load_signal::HeardLoadSignal;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutomataLinkState {
    pub partner_character: Option<Character>,
    /// The other game's hello or a state has arrived since the connect.
    pub partner_heard: bool,
    pub remote_input: RemoteInput,
    pub partner_load: Option<HeardLoadSignal>,
}

pub type NetShared = nc_hook::net::NetShared<BodySnapshot, PlayerState, AutomataLinkState>;
