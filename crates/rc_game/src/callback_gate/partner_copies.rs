use crate::callback_gate::{END_FOLLOW_EVENT_SUFFIX, GATED_EVENT_SUFFIX};

#[derive(Debug, Clone, Default)]
pub struct PartnerStartedCopies {
    families: Vec<(String, u16)>,
}

impl PartnerStartedCopies {
    pub const fn new() -> Self {
        Self {
            families: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.families.clear();
    }

    pub fn note(&mut self, name: &str, phase: u16) {
        self.families.retain(|(_, known)| *known == phase);
        self.families.push((name.to_owned(), phase));
    }

    pub fn contains(&mut self, name: &str, phase: u16) -> bool {
        self.families.retain(|(_, known)| *known == phase);
        let family = event_family(name);
        self.families
            .iter()
            .any(|(known, _)| event_family(known) == family)
    }

    pub fn take(&mut self, name: &str, phase: u16) -> bool {
        self.families.retain(|(_, known)| *known == phase);
        let family = event_family(name);
        let Some(index) = self
            .families
            .iter()
            .position(|(known, _)| event_family(known) == family)
        else {
            return false;
        };
        self.families.remove(index);
        true
    }
}

pub fn event_base(name: &str) -> &str {
    let trimmed = name.trim_end_matches(|character: char| character.is_ascii_digit());
    trimmed
        .strip_suffix(END_FOLLOW_EVENT_SUFFIX)
        .or_else(|| trimmed.strip_suffix(GATED_EVENT_SUFFIX))
        .unwrap_or(name)
}

pub fn event_family(name: &str) -> &str {
    let mut end = 0;
    for segment in name.split('_') {
        end += segment.len();
        let mut characters = segment.chars();
        let is_event_id = characters.next() == Some('a')
            && characters
                .next()
                .is_some_and(|digit| digit.is_ascii_digit());
        if is_event_id {
            return &name[..end];
        }
        end += 1;
    }
    event_base(name)
}
