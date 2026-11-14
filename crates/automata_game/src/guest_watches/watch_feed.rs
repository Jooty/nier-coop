use std::fmt;

/// What the watch drive reads before it feeds and places the hero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatchFeedReadings {
    pub stick_is_hooked: bool,
    pub host_state_is_fresh: bool,
    pub local_phase_id: i32,
    pub host_phase_id: Option<i32>,
    pub a_jump_is_pending: bool,
    pub local_free: bool,
    pub host_free: bool,
}

/// Why the hero is only held still this tick: nothing of the host is fed or corrected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchFeedRefusal {
    StickNotHooked,
    NoFreshHostState,
    OtherPhase { local: i32, host: i32 },
    JumpPending,
    LocalNotFree,
    HostNotFree,
}

impl fmt::Display for WatchFeedRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StickNotHooked => {
                write!(
                    f,
                    "the controller's stick update is not hooked, nothing can be fed"
                )
            }
            Self::NoFreshHostState => write!(f, "no fresh state of the host"),
            Self::OtherPhase { local, host } => write!(
                f,
                "this game is in phase {local:#X} and the host in {host:#X}"
            ),
            Self::JumpPending => write!(f, "a jump is pending or under way"),
            Self::LocalNotFree => write!(
                f,
                "this game holds its player (a cutscene, an event or a scene change)"
            ),
            Self::HostNotFree => write!(
                f,
                "the host's game holds its player (a cutscene, an event or a scene change)"
            ),
        }
    }
}

/// None when the host's input and place are fed to the hero this tick.
pub fn watch_feed_refusal(readings: WatchFeedReadings) -> Option<WatchFeedRefusal> {
    if !readings.stick_is_hooked {
        return Some(WatchFeedRefusal::StickNotHooked);
    }
    if !readings.host_state_is_fresh {
        return Some(WatchFeedRefusal::NoFreshHostState);
    }
    if let Some(host) = readings.host_phase_id
        && host != readings.local_phase_id
    {
        return Some(WatchFeedRefusal::OtherPhase {
            local: readings.local_phase_id,
            host,
        });
    }
    if readings.a_jump_is_pending {
        return Some(WatchFeedRefusal::JumpPending);
    }
    if !readings.local_free {
        return Some(WatchFeedRefusal::LocalNotFree);
    }
    if !readings.host_free {
        return Some(WatchFeedRefusal::HostNotFree);
    }
    None
}
