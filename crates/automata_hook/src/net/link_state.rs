use std::collections::VecDeque;

use automata_game::enemy_lead::EnemyLead;
use automata_game::partner::{BodySnapshot, RemoteInput};
use automata_proto::PlayerState;

use crate::net::partner_watch::PartnerWatch;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutomataLinkState {
    pub remote_input: RemoteInput,
    pub enemy_leads: VecDeque<EnemyLead>,
}

pub type NetShared =
    nc_hook::net::NetShared<BodySnapshot, PlayerState, PartnerWatch, AutomataLinkState>;
