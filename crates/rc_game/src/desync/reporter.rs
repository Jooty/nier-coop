use rc_proto::DesyncDigest;

use crate::desync::{DESYNC_DIGEST_INTERVAL_MS, DESYNC_IN_STEP_INTERVAL_MS};

#[derive(Debug, Default)]
pub struct DesyncReporter {
    last_sent_ms: Option<u64>,
    last_in_step_ms: Option<u64>,
}

impl DesyncReporter {
    pub fn digest_is_due(&mut self, now_ms: u64) -> bool {
        let due = self
            .last_sent_ms
            .is_none_or(|sent| now_ms.saturating_sub(sent) >= DESYNC_DIGEST_INTERVAL_MS);
        if due {
            self.last_sent_ms = Some(now_ms);
        }
        due
    }

    pub fn compare(
        &mut self,
        now_ms: u64,
        yours: &DesyncDigest,
        partner: &DesyncDigest,
        partner_position_seen: Option<[f32; 3]>,
    ) -> Vec<String> {
        let differences = yours.differences(partner, partner_position_seen);
        if !differences.is_empty() {
            return differences
                .into_iter()
                .map(|line| format!("desync: {line}"))
                .collect();
        }
        let due = self
            .last_in_step_ms
            .is_none_or(|logged| now_ms.saturating_sub(logged) >= DESYNC_IN_STEP_INTERVAL_MS);
        if !due {
            return Vec::new();
        }
        self.last_in_step_ms = Some(now_ms);
        vec!["desync: in step".to_owned()]
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
