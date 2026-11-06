use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Phase {
    Handshaking,
    Authenticating { nonce: [u8; 16] },
    Ready,
    Refused { drop_at: Instant },
}

pub(super) struct PeerSession {
    pub(super) phase: Phase,
    pub(super) connected_at: Instant,
    pub(super) last_heard: Instant,
    pub(super) last_ping_sent: Instant,
    pub(super) outstanding_ping: Option<(u32, Instant)>,
    pub(super) rtt: Option<Duration>,
    pub(super) slot: u8,
    pub(super) last_tick: Option<u32>,
    pub(super) guest_name: String,
    pub(super) pings_sent: u32,
    pub(super) pongs_received: u32,
}

impl PeerSession {
    pub(super) fn new(now: Instant) -> Self {
        Self {
            phase: Phase::Handshaking,
            connected_at: now,
            last_heard: now,
            last_ping_sent: now,
            outstanding_ping: None,
            rtt: None,
            slot: 0,
            last_tick: None,
            guest_name: String::new(),
            pings_sent: 0,
            pongs_received: 0,
        }
    }

    pub(super) fn accepts_tick(&mut self, tick: u32) -> bool {
        let newer = match self.last_tick {
            None => true,
            Some(last) => (tick.wrapping_sub(last) as i32) > 0,
        };
        if newer {
            self.last_tick = Some(tick);
        }
        newer
    }
}
