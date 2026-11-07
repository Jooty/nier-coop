use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::net::local_snapshot::LocalSnapshot;
use rc_game::readiness::{PartnerWatch, REPORT_STALE_MS};
use rc_game::story_calls::{PartnerPlace, Whereabouts};
use rc_proto::{GameReadiness, PlayerSettings, PlayerState, RejectReason, SessionOptions};

static NET_CLOCK_START: OnceLock<Instant> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NetState {
    #[default]
    WaitingForLauncher,
    LauncherLinked,
    Connected,
    Rejected(RejectReason),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemotePlayer {
    pub name: String,
    pub slot: u8,
    pub state: PlayerState,
    pub received_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct NetShared {
    pub state: NetState,
    pub link_port: Option<u16>,
    pub peer_name: Option<String>,
    pub rtt: Option<Duration>,
    pub direct_path: Option<bool>,
    pub remote: Option<RemotePlayer>,
    pub local: Option<LocalSnapshot>,
    pub local_slot: Option<u8>,
    pub options: SessionOptions,
    pub pending_settings: Option<PlayerSettings>,
    pub partner: PartnerWatch,
    pub local_readiness: GameReadiness,
    pub local_readiness_at: Option<Instant>,
}

pub type SharedNet = Arc<Mutex<NetShared>>;

impl NetState {
    pub fn label(&self) -> String {
        match self {
            Self::WaitingForLauncher => "waiting for the launcher".to_owned(),
            Self::LauncherLinked => "launcher linked, no other player".to_owned(),
            Self::Connected => "connected".to_owned(),
            Self::Rejected(reason) => format!("rejected ({reason:?})"),
            Self::Failed(message) => format!("failed ({message})"),
        }
    }
}

impl RemotePlayer {
    pub fn age(&self) -> Duration {
        self.received_at.elapsed()
    }
}

impl NetShared {
    pub fn partner_in_play(&self) -> bool {
        self.state == NetState::Connected && self.partner.is_in_play()
    }

    pub fn partner_joining(&self) -> bool {
        self.state == NetState::Connected && self.partner.is_joining()
    }

    /// Whether the partner, in play, stands where a shared scene on this game would include them.
    pub fn partner_place(&self) -> PartnerPlace {
        if !self.partner_in_play() {
            return PartnerPlace::Unknown;
        }
        PartnerPlace::judge(
            self.local.as_ref().map(|local| Whereabouts {
                phase: local.phase,
                position: local.position,
            }),
            self.remote.as_ref().map(|remote| Whereabouts {
                phase: remote.state.phase,
                position: remote.state.position,
            }),
        )
    }

    pub fn partner_counts_for_zone_rule(&self) -> bool {
        self.state == NetState::Connected && self.partner.counts_for_zone_rule(net_clock_ms())
    }

    pub fn effective_local_readiness(&self) -> GameReadiness {
        let fresh = self
            .local_readiness_at
            .is_some_and(|at| at.elapsed().as_millis() as u64 <= REPORT_STALE_MS);
        if fresh {
            self.local_readiness
        } else {
            GameReadiness::Loading
        }
    }
}

pub fn net_clock_ms() -> u64 {
    NET_CLOCK_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis() as u64
}
