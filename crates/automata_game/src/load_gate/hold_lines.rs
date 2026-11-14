use crate::load_gate::{PartnerLoad, StoryPlace};

pub fn partner_load_state_label(
    partner: &PartnerLoad,
    linked: bool,
    held_ms: u64,
    now_ms: u64,
) -> String {
    if partner.waiting {
        "load done and waiting".to_owned()
    } else if partner.in_play {
        "in play".to_owned()
    } else if !linked {
        "not linked".to_owned()
    } else if partner.never_heard {
        format!(
            "linked for {} s, no state message ever heard",
            held_ms / 1000
        )
    } else {
        format!(
            "loading (a body-less state heard {} ms ago)",
            now_ms.saturating_sub(partner.heard_at_ms)
        )
    }
}

pub fn load_hold_waiting_line(
    held_ms: u64,
    now_ms: u64,
    linked: bool,
    local_place: &StoryPlace,
    partner: &PartnerLoad,
    partner_place: &StoryPlace,
) -> String {
    format!(
        "load: end held for {held_ms} ms, this game {}, the partner {} at {}",
        local_place.describe(),
        partner_load_state_label(partner, linked, held_ms, now_ms),
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

pub fn campaign_loads_note(
    every_load_together: bool,
    start_together: bool,
    in_a_session: bool,
    loader_hooked: bool,
) -> String {
    let off = if !every_load_together {
        Some("the setting every_load_together is off")
    } else if !start_together {
        Some("the setting start_together is off")
    } else if !in_a_session {
        Some("there is no session")
    } else if !loader_hooked {
        Some("the end of the load cannot be held on this exe")
    } else {
        None
    };
    match off {
        Some(reason) => format!("load: later loads of a campaign are not held ({reason})"),
        None => "load: every later load of a campaign is held until both loads are done".to_owned(),
    }
}

pub fn campaign_load_began_line(place: &StoryPlace) -> String {
    format!(
        "load: a campaign load began, this game was at {}; its end is held for the partner",
        place.describe()
    )
}

pub fn campaign_load_not_held_line(phase_id: i32) -> String {
    format!("load: not held, phase {phase_id:#X} is a system phase")
}

pub fn campaign_load_ended_line(place: &StoryPlace) -> String {
    format!(
        "load: the campaign load ended, this game is at {}",
        place.describe()
    )
}

pub fn campaign_load_link_lost_line() -> String {
    "load: the link went down during a campaign load, the hold is let go".to_owned()
}
