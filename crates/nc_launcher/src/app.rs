mod actions;
mod events;
#[cfg(feature = "snapshots")]
mod fixture;
mod forms;
mod log_upload;
mod pages;
mod palette;
mod screen;
mod state;
mod style;
mod tasks;
mod updates;
mod window;

pub use actions::{ActionReport, run_install, run_uninstall};
pub use events::describe_event;
#[cfg(feature = "snapshots")]
pub use fixture::fixture_launcher;
pub use forms::{NetForms, precheck_failure};
pub use log_upload::LogUploader;
pub use pages::{Tab, draw};
pub use screen::Screen;
pub use state::{Launcher, Role};
pub use style::{apply_shared_spacing, heading_text, imgui_dark_visuals, install_heading_font};
pub use tasks::Task;
pub use updates::{
    RESTART_TAB_ARGUMENT, RESTART_TAB_LOGS, RESTART_TAB_SETTINGS, RESTART_UPDATED_ARGUMENT,
    UpdateState, Updater,
};
pub use window::run;

pub const WINDOW_TITLE: &str = "Nier Coop";
pub const WINDOW_WIDTH: f32 = 580.0;
pub const WINDOW_HEIGHT: f32 = 660.0;
pub const STEAM_RUN_URL_PREFIX: &str = "steam://rungameid/";
pub const GAME_POLL_SECONDS: f64 = 2.0;
