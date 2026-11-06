use std::collections::HashMap;
use std::time::Instant;

use crate::session::{ATTEMPT_WINDOW, GLOBAL_MAX_FAILURES, PEER_MAX_FAILURES};
use crate::transport::PeerId;

const PROOF_CONTEXT: &[u8] = b"rc-auth/1";

#[derive(Default)]
pub(super) struct AttemptLimiter {
    per_peer: HashMap<PeerId, Vec<Instant>>,
    all: Vec<Instant>,
}

impl AttemptLimiter {
    pub(super) fn is_locked(&mut self, peer: PeerId, now: Instant) -> bool {
        self.forget_old(now);
        let from_peer = self.per_peer.get(&peer).map_or(0, Vec::len);
        from_peer >= PEER_MAX_FAILURES || self.all.len() >= GLOBAL_MAX_FAILURES
    }

    pub(super) fn record_failure(&mut self, peer: PeerId, now: Instant) {
        self.per_peer.entry(peer).or_default().push(now);
        self.all.push(now);
    }

    fn forget_old(&mut self, now: Instant) {
        let fresh = |at: &Instant| now.saturating_duration_since(*at) < ATTEMPT_WINDOW;
        self.all.retain(fresh);
        self.per_peer
            .values_mut()
            .for_each(|times| times.retain(fresh));
        self.per_peer.retain(|_, times| !times.is_empty());
    }
}

pub fn password_proof(password: &str, nonce: &[u8; 16], guest: PeerId, host: PeerId) -> [u8; 32] {
    let mut message = Vec::with_capacity(PROOF_CONTEXT.len() + 16 + 64);
    message.extend_from_slice(PROOF_CONTEXT);
    message.extend_from_slice(nonce);
    message.extend_from_slice(&guest.0);
    message.extend_from_slice(&host.0);
    hmac_sha256::HMAC::mac(&message, password.as_bytes())
}

pub fn proofs_match(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

pub fn fresh_nonce() -> [u8; 16] {
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).expect("the operating system supplies random bytes");
    nonce
}
