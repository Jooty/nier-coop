use std::sync::Mutex;
use std::time::Duration;

use automata_game::event_follow::{EventId, PartnerReport};
use automata_game::load_gate::PARTNER_LOAD_SIGNAL_STALE_MS;

use crate::net::NetShared;

static LOCAL_PLAYING_EVENT: Mutex<Option<EventId>> = Mutex::new(None);

/// The story event every state message of this game says it plays from now on.
pub fn set_local_playing_event(event: Option<EventId>) {
    if let Ok(mut held) = LOCAL_PLAYING_EVENT.lock() {
        *held = event;
    }
}

pub fn local_playing_event() -> Option<EventId> {
    LOCAL_PLAYING_EVENT
        .lock()
        .map(|held| *held)
        .unwrap_or_default()
}

/// The partner's playing story event from its latest state, while that state is fresh.
pub fn partner_playing_event_of(shared: &NetShared) -> Option<EventId> {
    let latest = shared.partner_state()?;
    let fresh = latest.received_at.elapsed() <= Duration::from_millis(PARTNER_LOAD_SIGNAL_STALE_MS);
    fresh.then_some(latest.state.playing_event).flatten()
}

/// The partner's latest state report as a skip vote reads it, with its age.
pub fn partner_report_of(shared: &NetShared) -> Option<PartnerReport> {
    let latest = shared.partner_state()?;
    Some(PartnerReport {
        playing: latest.state.playing_event,
        unattended: latest.state.unattended,
        age_ms: u64::try_from(latest.received_at.elapsed().as_millis()).unwrap_or(u64::MAX),
    })
}
