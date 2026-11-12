mod actions;
mod events;
#[cfg(feature = "snapshots")]
mod fixture;
mod forms;
mod game_look;
mod log_upload;
mod pages;
mod palette;
mod parts;
mod screen;
mod state;
mod style;
mod tasks;
mod updates;
mod window;

pub use actions::{ActionReport, run_install, run_uninstall};
pub use events::describe_event;
#[cfg(feature = "snapshots")]
pub use fixture::{draw_parts_sheet, fixture_launcher};
pub use forms::{NetForms, precheck_failure};
pub use game_look::{GameLook, GameMark, TitleFace, game_look};
pub use log_upload::LogUploader;
pub use pages::{Tab, draw};
pub use screen::Screen;
pub use state::{Launcher, Role};
pub use style::{apply_launcher_look, heading_text, launcher_visuals};
pub use tasks::Task;
pub use updates::{
    RESTART_TAB_ARGUMENT, RESTART_TAB_LOGS, RESTART_TAB_SETTINGS, RESTART_UPDATED_ARGUMENT,
    UpdateState, Updater,
};
pub use window::run;

pub const WINDOW_TITLE: &str = "Nier Coop";
pub const WINDOW_WIDTH: f32 = 1180.0;
pub const WINDOW_HEIGHT: f32 = 760.0;
pub const WINDOW_MIN_WIDTH: f32 = 900.0;
pub const WINDOW_MIN_HEIGHT: f32 = 600.0;
pub const STEAM_RUN_URL_PREFIX: &str = "steam://rungameid/";
pub const GAME_POLL_SECONDS: f64 = 2.0;
