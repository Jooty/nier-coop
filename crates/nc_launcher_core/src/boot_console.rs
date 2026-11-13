mod console_clock;
mod console_script;

pub use console_clock::{ConsoleClock, ConsoleLineView, ConsoleView};
pub use console_script::{
    ConsoleScript, ConsoleStep, ConsoleTask, HostingFacts, JoiningFacts, StepEnd, hosting_script,
    joining_script,
};

pub const CONSOLE_LINE_TYPING_MS: f32 = 140.0;
pub const CONSOLE_SHORTEST_RUN_MS: f32 = 900.0;
pub const CONSOLE_LINE_COLUMNS: usize = 26;
pub const CONSOLE_LEADER: char = '.';
