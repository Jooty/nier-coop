use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use nc_hook::net::net_clock_ms;

use automata_game::load_gate::{
    LoadSignal, LocalLoadSignal, PARTNER_LOAD_SIGNAL_STALE_MS, PartnerLoad,
};
use automata_game::partner::BodySnapshot;

use crate::net::link_state::NetShared;

const NO_RELEASE_MOMENT: u64 = u64::MAX;

static LOCAL_WAITING: AtomicBool = AtomicBool::new(false);
static LOCAL_RELEASE_AT_MS: AtomicU64 = AtomicU64::new(NO_RELEASE_MOMENT);

/// The other player's newest word about its loading screen, and when it was heard on the net clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeardLoadSignal {
    pub signal: LoadSignal,
    pub heard_at_ms: u64,
}

/// What every state message of this game says about its loading screen from now on.
pub fn set_local_load_signal(signal: LocalLoadSignal) {
    LOCAL_WAITING.store(signal.waiting, Ordering::Release);
    LOCAL_RELEASE_AT_MS.store(
        signal.release_at_ms.unwrap_or(NO_RELEASE_MOMENT),
        Ordering::Release,
    );
}

pub fn local_load_signal() -> LoadSignal {
    let release_at_ms = LOCAL_RELEASE_AT_MS.load(Ordering::Acquire);
    LocalLoadSignal {
        without_body: false,
        waiting: LOCAL_WAITING.load(Ordering::Acquire),
        release_at_ms: (release_at_ms != NO_RELEASE_MOMENT).then_some(release_at_ms),
    }
    .at(net_clock_ms())
}

/// The snapshot a game publishes while it has no body to tell of: a real one always has a phase.
pub fn snapshot_without_body() -> BodySnapshot {
    BodySnapshot::default()
}

pub fn is_snapshot_without_body(snapshot: &BodySnapshot) -> bool {
    *snapshot == BodySnapshot::default()
}

pub fn partner_load_of(shared: &NetShared, now_ms: u64) -> PartnerLoad {
    let heard = shared.extras.partner_load;
    let fresh = heard
        .filter(|heard| now_ms.saturating_sub(heard.heard_at_ms) <= PARTNER_LOAD_SIGNAL_STALE_MS);
    PartnerLoad {
        waiting: fresh.is_some_and(|heard| heard.signal.waiting),
        in_play: fresh.is_some_and(|heard| !heard.signal.without_body),
        release_in_ms: heard.and_then(|heard| heard.signal.release_in_ms),
        heard_at_ms: heard.map_or(0, |heard| heard.heard_at_ms),
        round_trip_ms: shared
            .rtt
            .map(|rtt| u64::try_from(rtt.as_millis()).unwrap_or(u64::MAX)),
    }
}
