use replicant_proto::PartnerBody;

pub const BODY_RESEND_INTERVAL_MS: u64 = 1000;

#[derive(Debug, Clone, Copy, Default)]
pub struct BodyAnnouncer {
    last_sent: Option<(PartnerBody, u32, u64)>,
}

impl BodyAnnouncer {
    pub const fn new() -> Self {
        Self { last_sent: None }
    }

    pub fn is_due(&mut self, body: PartnerBody, phase_loads: u32, now_ms: u64) -> bool {
        let due = match self.last_sent {
            None => true,
            Some((sent, loads, _)) if sent != body || loads != phase_loads => true,
            Some((_, _, at_ms)) => now_ms.saturating_sub(at_ms) >= BODY_RESEND_INTERVAL_MS,
        };
        if due {
            self.last_sent = Some((body, phase_loads, now_ms));
        }
        due
    }
}
