use crate::boot_console::console_script::{ConsoleScript, StepEnd};
use crate::boot_console::{
    CONSOLE_LEADER, CONSOLE_LINE_COLUMNS, CONSOLE_LINE_TYPING_MS, CONSOLE_SHORTEST_RUN_MS,
};

/// When one boot console began and when each of its steps was first seen to end, in milliseconds on the
/// caller's clock.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsoleClock {
    started_ms: f64,
    ended_at_ms: Vec<Option<f64>>,
    skipped: bool,
}

/// One line as it stands: the text typed so far, and its status word once shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleLineView {
    pub typed: String,
    pub word: Option<&'static str>,
    pub failed: bool,
}

/// The console as it stands at one moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleView {
    pub lines: Vec<ConsoleLineView>,
    pub cursor_line: Option<usize>,
    pub error: Option<String>,
    pub failed: bool,
    pub lobby_due: bool,
    pub moving: bool,
}

impl ConsoleClock {
    pub fn new(now_ms: f64) -> Self {
        Self {
            started_ms: now_ms,
            ended_at_ms: Vec::new(),
            skipped: false,
        }
    }

    /// Notes the moment each step is first seen to have ended.
    pub fn observe(&mut self, script: &ConsoleScript, now_ms: f64) {
        self.ended_at_ms.resize(script.steps.len(), None);
        for (step, ended_at) in script.steps.iter().zip(&mut self.ended_at_ms) {
            if step.end != StepEnd::Running && ended_at.is_none() {
                *ended_at = Some(now_ms);
            }
        }
    }

    /// Shows every line as far as the work has really come, at once, from now on.
    pub fn skip(&mut self) {
        self.skipped = true;
    }

    pub fn view(&self, script: &ConsoleScript, now_ms: f64, motion: bool) -> ConsoleView {
        let typing = motion && !self.skipped;
        let elapsed = (now_ms - self.started_ms) as f32;
        let mut lines = Vec::new();
        let mut cursor_line = None;
        let mut moving = false;
        let mut failed = false;
        let mut line_start = 0.0_f32;
        let mut last_word_at = 0.0_f32;
        for (index, step) in script.steps.iter().enumerate() {
            if elapsed < line_start {
                moving = true;
                break;
            }
            let full = leader_text(step.label);
            let on_time_start = index as f32 * CONSOLE_LINE_TYPING_MS;
            // A line that starts late because real work held the one above appears whole.
            let typed_by = if typing && line_start <= on_time_start {
                line_start + CONSOLE_LINE_TYPING_MS
            } else {
                line_start
            };
            let share = if typed_by > line_start {
                ((elapsed - line_start) / (typed_by - line_start)).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let typed: String = full
                .chars()
                .take((share * full.chars().count() as f32).floor() as usize)
                .collect();
            let ended_at = self
                .ended_at_ms
                .get(index)
                .copied()
                .flatten()
                .filter(|_| step.end != StepEnd::Running)
                .map(|at| (at - self.started_ms) as f32);
            let word_at = ended_at.map(|at| at.max(typed_by));
            let shown = word_at.is_some_and(|at| elapsed >= at);
            if share < 1.0 || (word_at.is_some() && !shown) {
                moving = true;
            }
            let line_failed = step.end == StepEnd::Failed && shown;
            lines.push(ConsoleLineView {
                typed,
                word: shown.then_some(step.word),
                failed: line_failed,
            });
            if !shown {
                cursor_line = Some(index);
                break;
            }
            last_word_at = word_at.unwrap_or(line_start);
            if line_failed {
                failed = true;
                break;
            }
            line_start = last_word_at;
        }
        let all_shown = !failed
            && lines.len() == script.steps.len()
            && lines.iter().all(|line| line.word.is_some());
        let lobby_at = if typing {
            last_word_at.max(CONSOLE_SHORTEST_RUN_MS)
        } else {
            last_word_at
        };
        let lobby_due = all_shown && elapsed >= lobby_at;
        if all_shown && !lobby_due {
            moving = true;
        }
        ConsoleView {
            lines,
            cursor_line,
            error: failed.then(|| script.error.clone()).flatten(),
            failed,
            lobby_due,
            moving,
        }
    }
}

fn leader_text(label: &str) -> String {
    let used = label.chars().count() + 1;
    let leaders = CONSOLE_LINE_COLUMNS.saturating_sub(used).max(1);
    let mut text = String::with_capacity(used + leaders);
    text.push_str(label);
    text.push(' ');
    text.extend(std::iter::repeat_n(CONSOLE_LEADER, leaders));
    text
}
