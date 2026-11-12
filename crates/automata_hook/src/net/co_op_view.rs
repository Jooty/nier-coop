use automata_game::character::Character;
use nc_hook::menu::{CoOpStatus, PlayerLine};
use nc_hook::net::NetState;

use crate::net::link_state::NetShared;
use crate::net::partner_watch::NetSharedExt;
use crate::net::{LINK_NOT_STARTED_TEXT, UNNAMED_PARTNER_TEXT};

pub fn link_not_started_status() -> CoOpStatus {
    CoOpStatus {
        link: LINK_NOT_STARTED_TEXT.to_owned(),
        players: Vec::new(),
        ping_ms: None,
    }
}

pub fn co_op_status_of(shared: &NetShared, partner_setting: Character) -> CoOpStatus {
    let players = if shared.state == NetState::Connected {
        vec![PlayerLine {
            name: shared
                .partner_name()
                .map_or_else(|| UNNAMED_PARTNER_TEXT.to_owned(), str::to_owned),
            detail: partner_detail(shared, partner_setting),
        }]
    } else {
        Vec::new()
    };
    CoOpStatus {
        link: shared.state.label(),
        players,
        ping_ms: shared
            .rtt
            .map(|rtt| u32::try_from(rtt.as_millis()).unwrap_or(u32::MAX)),
    }
}

fn partner_detail(shared: &NetShared, partner_setting: Character) -> String {
    let character = match shared.partner_watch().character {
        Some(character) => character.launcher_id().to_ascii_uppercase(),
        None => format!(
            "{} (the link setting; the partner has not said yet)",
            partner_setting.launcher_id().to_ascii_uppercase()
        ),
    };
    match shared.partner_state() {
        Some(remote) if remote.state.load.without_body => character,
        Some(remote) => format!(
            "{character}, health {} of {}",
            remote.state.health, remote.state.max_health
        ),
        None => character,
    }
}
