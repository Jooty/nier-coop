use automata_game::load_gate::{PartnerLoad, StoryPlace};
use automata_game::travel::JumpRole;
use nc_hook::net::{NetState, net_clock_ms};

use crate::net::{SessionModeExt, handle, partner_load_of, partner_place_of};

pub(crate) struct LinkReading {
    pub role: JumpRole,
    pub partner: PartnerLoad,
    pub partner_place: StoryPlace,
    pub campaign_link_up: bool,
    pub partner_present: bool,
    pub partner_watching: bool,
}

pub(crate) fn link_reading() -> LinkReading {
    let Some(net) = handle() else {
        return LinkReading {
            role: JumpRole::Solo,
            partner: PartnerLoad::default(),
            partner_place: StoryPlace::default(),
            campaign_link_up: false,
            partner_present: false,
            partner_watching: false,
        };
    };
    net.update(|shared| LinkReading {
        role: JumpRole::of_session(
            shared.session_mode(),
            shared.state == NetState::Connected,
            shared.local_slot,
        ),
        partner: partner_load_of(shared, net_clock_ms()),
        partner_place: partner_place_of(shared),
        campaign_link_up: shared.campaign_link_is_up(),
        partner_present: shared.campaign_partner().is_some(),
        partner_watching: shared
            .campaign_partner()
            .and_then(|partner| partner.latest.as_ref())
            .is_some_and(|latest| latest.state.watching),
    })
}
