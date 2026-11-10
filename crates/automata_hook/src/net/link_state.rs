use automata_game::character::Character;
use automata_game::partner::{BodySnapshot, RemoteInput};
use automata_proto::PlayerState;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutomataLinkState {
    pub partner_character: Option<Character>,
    pub remote_input: RemoteInput,
}

pub type NetShared = nc_hook::net::NetShared<BodySnapshot, PlayerState, AutomataLinkState>;
