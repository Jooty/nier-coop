use crate::callback_gate::{
    END_FOLLOW_EVENT_SUFFIX, GATED_EVENT_SUFFIX, STARTED_FROM_PARTNER_EXPIRY_MS,
};

#[derive(Debug, Clone, Default)]
pub struct PartnerStartedCopies {
    bases: Vec<(String, u64)>,
}

impl PartnerStartedCopies {
    pub const fn new() -> Self {
        Self { bases: Vec::new() }
    }

    pub fn reset(&mut self) {
        self.bases.clear();
    }

    pub fn note(&mut self, name: &str, now_ms: u64) {
        let base = event_base(name);
        self.bases.retain(|(known, _)| known != base);
        self.bases.push((base.to_owned(), now_ms));
    }

    pub fn contains(&mut self, name: &str, now_ms: u64) -> bool {
        self.prune(now_ms);
        let base = event_base(name);
        self.bases.iter().any(|(known, _)| known == base)
    }

    pub fn take(&mut self, name: &str, now_ms: u64) -> bool {
        self.prune(now_ms);
        let base = event_base(name);
        let Some(index) = self.bases.iter().position(|(known, _)| known == base) else {
            return false;
        };
        self.bases.remove(index);
        true
    }

    fn prune(&mut self, now_ms: u64) {
        self.bases
            .retain(|(_, since)| now_ms.saturating_sub(*since) <= STARTED_FROM_PARTNER_EXPIRY_MS);
    }
}

pub fn event_base(name: &str) -> &str {
    let trimmed = name.trim_end_matches(|character: char| character.is_ascii_digit());
    trimmed
        .strip_suffix(END_FOLLOW_EVENT_SUFFIX)
        .or_else(|| trimmed.strip_suffix(GATED_EVENT_SUFFIX))
        .unwrap_or(name)
}
