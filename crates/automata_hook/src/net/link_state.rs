use std::collections::VecDeque;

use automata_game::enemy_lead::EnemyLead;
use automata_game::partner::{BodySnapshot, InputsBySlot};
use automata_proto::PlayerState;

use crate::net::partner_watch::PartnerWatch;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutomataLinkState {
    pub remote_inputs: InputsBySlot,
    pub enemy_leads: VecDeque<(u8, EnemyLead)>,
}

impl AutomataLinkState {
    pub fn player_joined(&mut self, slot: u8) {
        self.forget_player(slot);
    }

    pub fn player_left(&mut self, slot: u8) {
        self.forget_player(slot);
    }

    fn forget_player(&mut self, slot: u8) {
        self.remote_inputs.forget(slot);
        self.enemy_leads.retain(|(sender, _)| *sender != slot);
    }
}

pub type NetShared =
    nc_hook::net::NetShared<BodySnapshot, PlayerState, PartnerWatch, AutomataLinkState>;
