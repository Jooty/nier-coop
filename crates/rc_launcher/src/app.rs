mod actions;
mod forms;
mod state;
mod style;
mod tasks;
mod ui;
mod window;

pub use actions::{ActionReport, run_install, run_uninstall};
pub use forms::NetForms;
pub use state::Launcher;
pub use style::imgui_dark_visuals;
pub use tasks::Task;
pub use ui::{Tab, draw};
pub use window::run;

pub const WINDOW_TITLE: &str = "Replicant Co-op";
pub const WINDOW_WIDTH: f32 = 520.0;
pub const WINDOW_HEIGHT: f32 = 420.0;
pub const STEAM_RUN_URL_PREFIX: &str = "steam://rungameid/";
pub const GAME_POLL_SECONDS: f64 = 2.0;
