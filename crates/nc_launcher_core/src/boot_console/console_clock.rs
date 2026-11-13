use crate::boot_console::console_script::{ConsoleScript, StepEnd};
use crate::boot_console::{CONSOLE_LINE_TYPING_MS, CONSOLE_SHORTEST_RUN_MS};

/// When one boot console began and when each of its steps was first seen to end, in milliseconds on the
/// caller's clock.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsoleClock {
    started_ms: f64,
    ended_at_ms: Vec<Option<f64>>,
    skipped: bool,
}

/// What a console line is: a step with its status, a fact under the step above it (the last one under that
/// step or not), or the closing line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleLineKind {
    Step,
    Detail { last: bool },
    Closing,
}

/// One line as it stands: what it says, how much of it is typed (0 to 1), its status word once shown, whether
/// its status is still held by real work, and when it began and when its status showed, in milliseconds since
/// the console opened.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsoleLineView {
    pub kind: ConsoleLineKind,
    pub label: &'static str,
    pub value: Option<String>,
    pub typed_share: f32,
    pub word: Option<&'static str>,
    pub failed: bool,
    pub waiting: bool,
    pub began_ms: f32,
    pub closed_ms: Option<f32>,
}

/// The console as it stands at one moment, with the steps closed so far out of the steps the meter counts.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsoleView {
    pub lines: Vec<ConsoleLineView>,
    pub cursor_line: Option<usize>,
    pub error: Option<String>,
    pub failed: bool,
    pub lobby_due: bool,
    pub moving: bool,
    pub closed_steps: usize,
    pub counted_steps: usize,
}

struct Typing {
    on: bool,
    elapsed: f32,
    line_start: f32,
    lines_before: usize,
}

impl Typing {
    fn typed_by(&self) -> f32 {
        let on_time_start = self.lines_before as f32 * CONSOLE_LINE_TYPING_MS;
        // A line that starts late because real work held the one above appears whole.
        if self.on && self.line_start <= on_time_start {
            self.line_start + CONSOLE_LINE_TYPING_MS
        } else {
            self.line_start
        }
    }

    fn share(&self, typed_by: f32) -> f32 {
        if typed_by > self.line_start {
            ((self.elapsed - self.line_start) / (typed_by - self.line_start)).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }
}

impl ConsoleClock {
    pub fn new(now_ms: f64) -> Self {
        Self {
            started_ms: now_ms,
            ended_at_ms: Vec::new(),
            skipped: false,
        }
    }

    pub fn started_ms(&self) -> f64 {
        self.started_ms
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
        let mut clock = Typing {
            on: typing,
            elapsed,
            line_start: 0.0,
            lines_before: 0,
        };
        let mut lines = Vec::new();
        let mut cursor_line = None;
        let mut moving = false;
        let mut failed = false;
        let mut closed_steps = 0;
        let mut last_word_at = 0.0_f32;
        let closing_index = script.steps.len().saturating_sub(1);
        'steps: for (index, step) in script.steps.iter().enumerate() {
            if elapsed < clock.line_start {
                moving = true;
                break;
            }
            let typed_by = clock.typed_by();
            let share = clock.share(typed_by);
            let word_at = self
                .ended_at_ms
                .get(index)
                .copied()
                .flatten()
                .filter(|_| step.end != StepEnd::Running)
                .map(|at| ((at - self.started_ms) as f32).max(typed_by));
            let shown = word_at.is_some_and(|at| elapsed >= at);
            if share < 1.0 || (word_at.is_some() && !shown) {
                moving = true;
            }
            let line_failed = step.end == StepEnd::Failed && shown;
            lines.push(ConsoleLineView {
                kind: if index == closing_index {
                    ConsoleLineKind::Closing
                } else {
                    ConsoleLineKind::Step
                },
                label: step.label,
                value: step.value.clone(),
                typed_share: share,
                word: shown.then_some(step.word),
                failed: line_failed,
                waiting: share >= 1.0 && !shown,
                began_ms: clock.line_start,
                closed_ms: word_at.filter(|_| shown),
            });
            clock.lines_before += 1;
            if !shown {
                cursor_line = Some(lines.len() - 1);
                break;
            }
            last_word_at = word_at.unwrap_or(clock.line_start);
            clock.line_start = last_word_at;
            if line_failed {
                failed = true;
                break;
            }
            if index != closing_index {
                closed_steps += 1;
            }
            let detail_count = step.details.len();
            for (detail_index, detail) in step.details.iter().enumerate() {
                if elapsed < clock.line_start {
                    moving = true;
                    break 'steps;
                }
                let typed_by = clock.typed_by();
                let share = clock.share(typed_by);
                lines.push(ConsoleLineView {
                    kind: ConsoleLineKind::Detail {
                        last: detail_index + 1 == detail_count,
                    },
                    label: detail.label,
                    value: Some(detail.value.clone()),
                    typed_share: share,
                    word: None,
                    failed: false,
                    waiting: false,
                    began_ms: clock.line_start,
                    closed_ms: None,
                });
                clock.lines_before += 1;
                if share < 1.0 {
                    moving = true;
                    cursor_line = Some(lines.len() - 1);
                    break 'steps;
                }
                last_word_at = typed_by;
                clock.line_start = typed_by;
            }
        }
        let all_shown = !failed
            && cursor_line.is_none()
            && lines.len() == line_count(script)
            && lines.last().is_some_and(|line| line.word.is_some());
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
            closed_steps,
            counted_steps: script.counted_steps(),
        }
    }
}

fn line_count(script: &ConsoleScript) -> usize {
    script.steps.iter().map(|step| 1 + step.details.len()).sum()
}
