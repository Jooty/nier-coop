use std::sync::{Mutex, PoisonError};

use automata_game::partner_hud::{TagInputs, shown_name, tag_hidden_reason};
use nc_hook::diagnostics::log_line;
use nc_hook::net::NetState;

use crate::hud::TAG_STATE_LINES_LOGGED_AT_MOST;
use crate::hud::co_op_log::note_session;
use crate::hud::install::calls;
use crate::net::{self, NetShared, local_in_play};
use crate::partner_driver::held_partner_body;

/// Where the other player is, as the last game tick knew it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PartnerView {
    pub name: String,
    pub position: [f32; 3],
    pub local_position: [f32; 3],
    pub distance: f32,
}

struct TagStateLog {
    last: Option<Option<&'static str>>,
    lines: u32,
}

static PARTNER_VIEW: Mutex<Option<PartnerView>> = Mutex::new(None);
static TAG_STATE_LOG: Mutex<TagStateLog> = Mutex::new(TagStateLog {
    last: None,
    lines: 0,
});

/// Game thread, once a frame: posts the co-op lines and works out whether and where the
/// partner's tag and map dot are to be drawn.
pub(crate) fn tick() {
    let Some(net) = net::handle() else {
        return;
    };
    let shared = net.view();
    let name = shown_name(shared.peer_name.as_deref().unwrap_or_default());
    note_session(&shared, &name);
    let hidden = tag_hidden_reason(&tag_inputs(&shared));
    log_tag_state_change(hidden);
    let view = match hidden {
        Some(_) => None,
        None => partner_view_of(&shared, name),
    };
    *PARTNER_VIEW.lock().unwrap_or_else(PoisonError::into_inner) = view;
}

pub(super) fn partner_view() -> Option<PartnerView> {
    PARTNER_VIEW
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

fn tag_inputs(shared: &NetShared) -> TagInputs {
    let (local, remote) = (shared.local.as_ref(), shared.remote.as_ref());
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

fn partner_view_of(shared: &NetShared, name: String) -> Option<PartnerView> {
    let local_position = shared.local.as_ref()?.position;
    let told_position = shared.remote.as_ref()?.state.position;
    // The body the other player drives here is what the eye sees; the told position is the
    // fallback while no body is held.
    let position = held_partner_body()
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
        name,
        position,
        local_position,
        distance: squared.sqrt(),
    })
}

fn log_tag_state_change(hidden: Option<&'static str>) {
    let mut log = TAG_STATE_LOG.lock().unwrap_or_else(PoisonError::into_inner);
    if log.last == Some(hidden) || log.lines >= TAG_STATE_LINES_LOGGED_AT_MOST {
        return;
    }
    log.last = Some(hidden);
    log.lines += 1;
    match hidden {
        Some(reason) => log_line(&format!("partner tag: hidden ({reason})")),
        None => log_line("partner tag: shown"),
    }
}
