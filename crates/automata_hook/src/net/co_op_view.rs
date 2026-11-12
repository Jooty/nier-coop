use automata_game::character::Character;
use automata_proto::PlayerState;
use nc_hook::menu::{CoOpStatus, PlayerLine};
use nc_hook::net::{NetState, Player};

use crate::net::link_state::NetShared;
use crate::net::partner_watch::PartnerWatch;
use crate::net::session_mode_view::SessionModeExt;
use crate::net::shown_players::ShownPlayersExt;
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
        shared
            .shown_others()
            .into_iter()
            .map(|player| PlayerLine {
                name: if player.name.is_empty() {
                    UNNAMED_PARTNER_TEXT.to_owned()
                } else {
                    player.name.clone()
                },
                detail: player_detail(shared, player, partner_setting),
            })
            .collect()
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

fn player_detail(
    shared: &NetShared,
    player: &Player<PlayerState, PartnerWatch>,
    partner_setting: Character,
) -> String {
    let character = match player.watch.character {
        Some(character) => character.launcher_id().to_ascii_uppercase(),
        None if shared.session_mode().story_rules_apply() => format!(
            "{} (the link setting; the partner has not said yet)",
            partner_setting.launcher_id().to_ascii_uppercase()
        ),
        None => "character not told yet".to_owned(),
    };
    match player.latest.as_ref() {
        Some(remote) if remote.state.load.without_body => character,
        Some(remote) => format!(
            "{character}, health {} of {}",
            remote.state.health, remote.state.max_health
        ),
        None => character,
    }
}
