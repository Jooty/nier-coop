use crate::loading_text::roll::next_roll;
use crate::loading_text::{
    LoadingChoice, SHOW_ONE_LOADING_SCREEN_IN, SessionPartner, clean_partner_name, fill_partner,
};

const FIRST_ROLL_STATE: u64 = 0x4E49_4552_434F_4F50;

#[derive(Debug, Clone)]
pub struct LoadingText {
    lines: &'static [&'static str],
    enabled: bool,
    show_one_in: u64,
    last_shown: Option<usize>,
    roll_state: u64,
}

impl LoadingText {
    pub const fn new(lines: &'static [&'static str]) -> Self {
        Self {
            lines,
            enabled: true,
            show_one_in: SHOW_ONE_LOADING_SCREEN_IN,
            last_shown: None,
            roll_state: FIRST_ROLL_STATE,
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn reseed(&mut self, seed: u64) {
        self.roll_state ^= seed;
    }

    pub fn choose(&mut self, partner: SessionPartner<'_>) -> Option<LoadingChoice> {
        let roll = next_roll(&mut self.roll_state);
        self.choose_with_roll(partner, roll)
    }

    pub fn choose_with_roll(
        &mut self,
        partner: SessionPartner<'_>,
        roll: u64,
    ) -> Option<LoadingChoice> {
        if !self.enabled || matches!(partner, SessionPartner::NoSession) {
            return None;
        }
        if !roll.is_multiple_of(self.show_one_in) {
            return None;
        }
        let name = match partner {
            SessionPartner::Named(name) => clean_partner_name(name),
            _ => None,
        };
        let mut candidates: Vec<(usize, String)> = self
            .lines
            .iter()
            .enumerate()
            .filter_map(|(index, template)| {
                fill_partner(template, name.as_deref()).map(|text| (index, text))
            })
            .collect();
        if candidates.len() > 1 {
            candidates.retain(|(index, _)| Some(*index) != self.last_shown);
        }
        let count = u64::try_from(candidates.len())
            .ok()
            .filter(|count| *count > 0)?;
        let picked = usize::try_from((roll / self.show_one_in) % count).ok()?;
        let (line_index, text) = candidates.swap_remove(picked);
        self.last_shown = Some(line_index);
        Some(LoadingChoice {
            line_index,
            text,
            partner_name_chars: name.map_or(0, |name| name.chars().count()),
        })
    }
}
