use crate::event_follow::EventId;
use crate::minute_line_budget::MinuteLineBudget;
use crate::partner_hud::PAUSE_WORD_LINES_LOGGED_PER_MINUTE;

const PAUSE_WORD_TOPIC: &str = "pause word";

#[derive(Debug, Clone, PartialEq)]
pub struct PauseWordFacts {
    pub pause_state_word: Option<i32>,
    pub playing_event: Option<EventId>,
    pub story_place: String,
}

#[derive(Debug, Clone)]
pub struct PauseWordWatch {
    last: Option<bool>,
    budget: MinuteLineBudget,
}

impl Default for PauseWordWatch {
    fn default() -> Self {
        Self {
            last: None,
            budget: MinuteLineBudget::new(PAUSE_WORD_LINES_LOGGED_PER_MINUTE),
        }
    }
}

impl PauseWordWatch {
    pub fn note(
        &mut self,
        now_ms: u64,
        paused: bool,
        facts: impl FnOnce() -> PauseWordFacts,
    ) -> Vec<String> {
        let before = self.last.replace(paused);
        match before {
            Some(old) if old != paused => {
                let line = pause_word_line(old, paused, &facts());
                self.budget.lines_for(now_ms, PAUSE_WORD_TOPIC, line)
            }
            _ => Vec::new(),
        }
    }
}

pub fn pause_word_line(old: bool, new: bool, facts: &PauseWordFacts) -> String {
    let word = match facts.pause_state_word {
        Some(word) => format!("{word:#X}"),
        None => "not read".to_owned(),
    };
    let event = match facts.playing_event {
        Some(event) => format!("an event plays ({}/{}/{})", event.kind, event.id, event.sub),
        None => "no event plays".to_owned(),
    };
    format!(
        "pause word: the word read as \"paused or in a menu\" went from {old} to {new}; pause state word {word}; {event}; story place {}",
        facts.story_place
    )
}
