use crate::load_gate::{PartnerLoad, StoryPlace};

pub fn partner_load_state_label(partner: &PartnerLoad) -> &'static str {
    if partner.waiting {
        "load done and waiting"
    } else if partner.in_play {
        "in play"
    } else {
        "loading or silent"
    }
}

pub fn load_hold_waiting_line(
    held_ms: u64,
    local_place: &StoryPlace,
    partner: &PartnerLoad,
    partner_place: &StoryPlace,
) -> String {
    format!(
        "load: end held for {held_ms} ms, this game {}, the partner {} at {}",
        local_place.describe(),
        partner_load_state_label(partner),
        partner_place.describe()
    )
}

pub fn alone_places_line(local_place: &StoryPlace, partner_place: &StoryPlace) -> String {
    format!(
        "load: went on alone, this game {}, the partner {}",
        local_place.describe(),
        partner_place.describe()
    )
}
