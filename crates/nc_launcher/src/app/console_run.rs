use nc_launcher_core::{ConsoleClock, ConsoleTask};

/// The boot console on screen: which wait it covers, and its clock.
pub struct ConsoleRun {
    pub task: ConsoleTask,
    pub clock: ConsoleClock,
}
