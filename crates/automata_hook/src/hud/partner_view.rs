use std::sync::{Mutex, PoisonError};

use automata_game::minute_line_budget::MinuteLineBudget;
use automata_game::partner_hud::{TagInputs, shown_name, tag_hidden_reason};
use automata_proto::PlayerState;
use nc_hook::net::{NetState, Player};

use crate::budgeted_log::log_owned_within;
use crate::extra_bodies::body_of_player;
use crate::hud::TAG_STATE_LINES_LOGGED_PER_MINUTE;
use crate::hud::co_op_log::note_session;
use crate::hud::install::calls;
use crate::net::{self, NetShared, PartnerWatch, ShownPlayersExt, local_in_play};

/// Where the other player is, as the last game tick knew it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PartnerView {
    pub slot: u8,
    pub name: String,
    pub position: [f32; 3],
    pub local_position: [f32; 3],
    pub distance: f32,
}

struct TagStateLog {
    last: Option<Option<&'static str>>,
    lines: MinuteLineBudget,
}

static PARTNER_VIEWS: Mutex<Vec<PartnerView>> = Mutex::new(Vec::new());
static TAG_STATE_LOG: Mutex<TagStateLog> = Mutex::new(TagStateLog {
    last: None,
    lines: MinuteLineBudget::new(TAG_STATE_LINES_LOGGED_PER_MINUTE),
});

/// Game thread, once a frame: posts the co-op lines and works out whether and where each
/// other player's tag and map dot are to be drawn.
pub(crate) fn tick() {
    let Some(net) = net::handle() else {
        return;
    };
    let shared = net.view();
    let players = shared.shown_others();
    note_session(&shared, &players);
    let mut views = Vec::new();
    if players.is_empty() {
        log_tag_state_change(tag_hidden_reason(&tag_inputs(&shared, None)));
    }
    for (index, player) in players.into_iter().enumerate() {
        let hidden = tag_hidden_reason(&tag_inputs(&shared, Some(player)));
        if index == 0 {
            log_tag_state_change(hidden);
        }
        if hidden.is_none() {
            views.extend(partner_view_of(&shared, player, shown_name(&player.name)));
        }
    }
    *PARTNER_VIEWS.lock().unwrap_or_else(PoisonError::into_inner) = views;
}

pub(super) fn partner_views() -> Vec<PartnerView> {
    PARTNER_VIEWS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

fn tag_inputs(shared: &NetShared, player: Option<&Player<PlayerState, PartnerWatch>>) -> TagInputs {
    let (local, remote) = (
        shared.local.as_ref(),
        player.and_then(|player| player.latest.as_ref()),
    );
    TagInputs {
        // Each overlay piece checks its own setting; the view is wanted while any of them is on.
        enabled: true,
        connected: shared.state == NetState::Connected,
        partner_state_age_ms: remote.map(|remote| remote.age().as_millis() as u64),
        same_place: local
            .zip(remote)
            .is_some_and(|(local, remote)| local.phase_id == remote.state.phase_id),
        local_in_play: local.is_some() && local_in_play(),
        local_free: local.is_some_and(|local| local.free),
        local_paused: local.is_some_and(|local| local.paused),
    }
}

fn partner_view_of(
    shared: &NetShared,
    player: &Player<PlayerState, PartnerWatch>,
    name: String,
) -> Option<PartnerView> {
    let local_position = shared.local.as_ref()?.position;
    let told_position = player.latest.as_ref()?.state.position;
    // The body the other player drives here is what the eye sees; the told position is the
    // fallback while no body is held.
    let position = body_of_player(player.slot)
        .zip(calls())
        .and_then(|(body, calls)| calls.read_body_position(body).ok())
        .map_or(told_position, |position| {
            [position[0], position[1], position[2]]
        });
    let squared: f32 = position
        .iter()
        .zip(local_position)
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    squared.is_finite().then(|| PartnerView {
        slot: player.slot,
        name,
        position,
        local_position,
        distance: squared.sqrt(),
    })
}

fn log_tag_state_change(hidden: Option<&'static str>) {
    let mut log = TAG_STATE_LOG.lock().unwrap_or_else(PoisonError::into_inner);
    if log.last == Some(hidden) {
        return;
    }
    log.last = Some(hidden);
    let line = match hidden {
        Some(reason) => format!("partner tag: hidden ({reason})"),
        None => "partner tag: shown".to_owned(),
    };
    log_owned_within(&mut log.lines, "partner tag", line);
}
