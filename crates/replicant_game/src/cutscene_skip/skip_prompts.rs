/// A skip both players vote on: a prompt page whose answer skips a sequence both are in, or a
/// pad skip callback (`prompt` is then the callback id the script registers it under).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkipPrompt {
    pub prompt: u16,
    pub shows_page: bool,
    pub skip_answer: &'static str,
    pub other_answer: Option<&'static str>,
    pub ended_by: &'static [&'static str],
    /// The functions that start the sequence in a game; empty when no start is known, and then
    /// any partner in the same place takes part in the vote.
    pub started_by: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptAnswer {
    Skip,
    Other,
}

pub const PROMPTS_WHOSE_ANSWER_SKIPS_A_SHARED_SEQUENCE: [SkipPrompt; 2] = [
    SkipPrompt {
        prompt: 2437,
        shows_page: true,
        skip_answer: "EID_1410_a0100g_16_GuideSkip_01",
        other_answer: Some("EID_1410_a0100g_16_GuideSkip_02"),
        ended_by: &[],
        started_by: &[],
    },
    SkipPrompt {
        prompt: 5410,
        shows_page: false,
        skip_answer: "Put_Boat_04",
        other_answer: None,
        ended_by: &["Put_Boat_03", "Put_Boat_05"],
        started_by: &["Put_Boat_02"],
    },
];

impl SkipPrompt {
    pub fn answer_of(&self, callback: &str) -> Option<PromptAnswer> {
        if callback == self.skip_answer {
            Some(PromptAnswer::Skip)
        } else if self.other_answer == Some(callback) {
            Some(PromptAnswer::Other)
        } else {
            None
        }
    }

    pub const fn callback_of(&self, answer: PromptAnswer) -> Option<&'static str> {
        match answer {
            PromptAnswer::Skip => Some(self.skip_answer),
            PromptAnswer::Other => self.other_answer,
        }
    }

    pub fn is_ended_by(&self, callback: &str) -> bool {
        self.ended_by.contains(&callback)
    }
}

/// The skip prompt whose option tag launches `callback`, and which option that is.
pub fn skip_prompt_answer(callback: &str) -> Option<(SkipPrompt, PromptAnswer)> {
    PROMPTS_WHOSE_ANSWER_SKIPS_A_SHARED_SEQUENCE
        .iter()
        .find_map(|prompt| prompt.answer_of(callback).map(|answer| (*prompt, answer)))
}

pub fn skip_prompt(prompt: u16) -> Option<SkipPrompt> {
    PROMPTS_WHOSE_ANSWER_SKIPS_A_SHARED_SEQUENCE
        .iter()
        .find(|row| row.prompt == prompt)
        .copied()
}

/// The skip whose sequence `callback` ends on its own (the ride arrives), if any.
pub fn skip_prompt_ended_by(callback: &str) -> Option<SkipPrompt> {
    PROMPTS_WHOSE_ANSWER_SKIPS_A_SHARED_SEQUENCE
        .iter()
        .find(|row| row.is_ended_by(callback))
        .copied()
}
