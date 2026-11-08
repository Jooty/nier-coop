use crate::callback_gate::{
    END_FOLLOW_EVENT_SUFFIX, GATED_EVENT_SUFFIX, STARTED_FROM_PARTNER_EXPIRY_MS,
    TALK_OWNER_EXPIRY_MS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TalkStart {
    RunHere,
    PartnerAway,
    PartnerOwns { had: String },
}

#[derive(Debug, Clone, Default)]
pub struct TalkOwner {
    partner_talks: Vec<(String, u64)>,
}

#[derive(Debug, Clone, Default)]
pub struct PartnerStartedCopies {
    bases: Vec<(String, u64)>,
}

impl TalkOwner {
    pub const fn new() -> Self {
        Self {
            partner_talks: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.partner_talks.clear();
    }

    pub fn note_partner_end(&mut self, end_name: &str, now_ms: u64) {
        let base = event_base(end_name);
        if talk_stem(base).is_none() {
            return;
        }
        self.partner_talks.retain(|(had, _)| had != base);
        self.partner_talks.push((base.to_owned(), now_ms));
    }

    pub fn start_verdict(
        &mut self,
        start_name: &str,
        partner_in_same_play: bool,
        now_ms: u64,
    ) -> TalkStart {
        self.partner_talks
            .retain(|(_, since)| now_ms.saturating_sub(*since) <= TALK_OWNER_EXPIRY_MS);
        if !partner_in_same_play {
            return TalkStart::PartnerAway;
        }
        let base = event_base(start_name);
        let Some(stem) = talk_stem(base) else {
            return TalkStart::RunHere;
        };
        let had = self
            .partner_talks
            .iter()
            .find(|(had, _)| had != base && talk_stem(had) == Some(stem));
        match had {
            Some((had, _)) => TalkStart::PartnerOwns { had: had.clone() },
            None => {
                self.partner_talks
                    .retain(|(had, _)| talk_stem(had) != Some(stem));
                TalkStart::RunHere
            }
        }
    }
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

pub fn talk_stem(base: &str) -> Option<&str> {
    let marker = base.rmatch_indices("_a").find(|(index, _)| {
        base[index + 2..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
    })?;
    let stem = &base[..marker.0];
    let last_segment = stem.rsplit('_').next()?;
    let mut characters = last_segment.chars();
    let is_npc_segment =
        characters.next() == Some('S') && characters.next().is_some_and(|c| c.is_ascii_digit());
    is_npc_segment.then_some(stem)
}
