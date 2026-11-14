use crate::cutscene_skip::{PROMPTS_WHOSE_ANSWER_SKIPS_A_SHARED_SEQUENCE, SkipPrompt, skip_prompt};

/// This game entered (`inside`) or left the sequence of skip prompt `prompt`, by running `by`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceChange {
    pub prompt: u16,
    pub inside: bool,
    pub by: &'static str,
}

/// Which skip prompt's sequence each game is in, from the functions this game runs and the
/// partner's reports. Only a player in the sequence has a say in its skip vote.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PromptSequences {
    phase: Option<u16>,
    here: Option<u16>,
    partner: Option<u16>,
}

impl PromptSequences {
    pub const fn new() -> Self {
        Self {
            phase: None,
            here: None,
            partner: None,
        }
    }

    /// This game's place now: a new place ends both players' sequences as this game knows them.
    pub fn enter_phase(&mut self, phase: Option<u16>) {
        if self.phase != phase {
            *self = Self {
                phase,
                ..Self::new()
            };
        }
    }

    /// A function ran in this game; the change, if any, is sent to the partner.
    pub fn ran_here(&mut self, function: &str) -> Option<SequenceChange> {
        if let Some((prompt, by)) = started_by(function) {
            if self.here == Some(prompt.prompt) {
                return None;
            }
            self.here = Some(prompt.prompt);
            return Some(SequenceChange {
                prompt: prompt.prompt,
                inside: true,
                by,
            });
        }
        let prompt = skip_prompt(self.here?)?;
        let by = [prompt.skip_answer]
            .into_iter()
            .chain(prompt.other_answer)
            .chain(prompt.ended_by.iter().copied())
            .find(|name| *name == function)?;
        self.here = None;
        Some(SequenceChange {
            prompt: prompt.prompt,
            inside: false,
            by,
        })
    }

    pub fn partner_reported(&mut self, prompt: u16, inside: bool) {
        if inside {
            self.partner = Some(prompt);
        } else if self.partner == Some(prompt) {
            self.partner = None;
        }
    }

    pub fn partner_inside(&self, prompt: SkipPrompt) -> bool {
        prompt.started_by.is_empty() || self.partner == Some(prompt.prompt)
    }

    pub fn here_inside(&self, prompt: SkipPrompt) -> bool {
        prompt.started_by.is_empty() || self.here == Some(prompt.prompt)
    }

    pub fn forget_partner(&mut self) {
        self.partner = None;
    }
}

fn started_by(function: &str) -> Option<(SkipPrompt, &'static str)> {
    PROMPTS_WHOSE_ANSWER_SKIPS_A_SHARED_SEQUENCE
        .iter()
        .find_map(|prompt| {
            prompt
                .started_by
                .iter()
                .find(|start| **start == function)
                .map(|start| (*prompt, *start))
        })
}
