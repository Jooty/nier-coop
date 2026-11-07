mod actions;
mod events;
mod forms;
mod pages;
mod state;
mod style;
mod tasks;
mod updates;
mod window;

pub use actions::{ActionReport, run_install, run_uninstall};
pub use events::describe_event;
pub use forms::{NetForms, precheck_failure};
pub use pages::{Tab, draw};
pub use state::{Launcher, Role};
pub use style::{apply_shared_spacing, imgui_dark_visuals};
pub use tasks::Task;
pub use updates::{
    RESTART_TAB_ARGUMENT, RESTART_TAB_SETTINGS, RESTART_UPDATED_ARGUMENT, UpdateState, Updater,
};
pub use window::run;

pub const WINDOW_TITLE: &str = "Replicant Co-op";
pub const WINDOW_WIDTH: f32 = 520.0;
pub const WINDOW_HEIGHT: f32 = 660.0;
pub const STEAM_RUN_URL_PREFIX: &str = "steam://rungameid/";
pub const GAME_POLL_SECONDS: f64 = 2.0;
