pub mod character_setting;
pub mod log_settings;
pub mod menu;
pub mod start_setting;

#[cfg(windows)]
mod actions;
#[cfg(windows)]
mod entry;
#[cfg(windows)]
mod game_calls;
#[cfg(windows)]
mod game_hooks;
#[cfg(windows)]
mod overlay;
#[cfg(windows)]
mod proxy;
#[cfg(windows)]
mod start_driver;

pub const BUILD_COMMIT: &str = env!("NC_BUILD_COMMIT");
