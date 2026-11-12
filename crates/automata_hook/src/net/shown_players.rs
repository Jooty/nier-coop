use automata_game::mode::SessionMode;
use automata_game::partner::{PlayerEntry, player_entries};
use automata_proto::PlayerState;
use nc_hook::net::Player;

use crate::net::link_state::NetShared;
use crate::net::partner_watch::PartnerWatch;
use crate::net::session_mode_view::SessionModeExt;

pub trait ShownPlayersExt {
    /// The players the screen and the menu show one entry for, in slot order: the campaign's
    /// partner alone, or every other player in free roam.
    fn shown_others(&self) -> Vec<&Player<PlayerState, PartnerWatch>>;
}

impl ShownPlayersExt for NetShared {
    fn shown_others(&self) -> Vec<&Player<PlayerState, PartnerWatch>> {
        let mode = self.session_mode();
        let candidates: Vec<&Player<PlayerState, PartnerWatch>> = match mode {
            SessionMode::Campaign => self.campaign_partner().into_iter().collect(),
            SessionMode::FreeRoam => self.others().collect(),
        };
        let entries: Vec<PlayerEntry> = candidates
            .iter()
            .map(|player| PlayerEntry {
                slot: player.slot,
                name: player.name.clone(),
            })
            .collect();
        player_entries(mode, &entries)
            .into_iter()
            .filter_map(|entry| {
                candidates
                    .iter()
                    .find(|player| player.slot == entry.slot)
                    .copied()
            })
            .collect()
    }
}
