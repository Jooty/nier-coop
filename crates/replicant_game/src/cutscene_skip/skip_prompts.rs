#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkipPrompt {
    pub prompt: u16,
    pub skip_answer: &'static str,
    pub other_answer: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptAnswer {
    Skip,
    Other,
}

pub const PROMPTS_WHOSE_ANSWER_SKIPS_A_SHARED_SEQUENCE: [SkipPrompt; 1] = [SkipPrompt {
    prompt: 2437,
    skip_answer: "EID_1410_a0100g_16_GuideSkip_01",
    other_answer: "EID_1410_a0100g_16_GuideSkip_02",
}];

impl SkipPrompt {
    pub fn answer_of(&self, callback: &str) -> Option<PromptAnswer> {
        if callback == self.skip_answer {
            Some(PromptAnswer::Skip)
        } else if callback == self.other_answer {
            Some(PromptAnswer::Other)
        } else {
            None
        }
    }

    pub const fn callback_of(&self, answer: PromptAnswer) -> &'static str {
        match answer {
            PromptAnswer::Skip => self.skip_answer,
            PromptAnswer::Other => self.other_answer,
        }
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
