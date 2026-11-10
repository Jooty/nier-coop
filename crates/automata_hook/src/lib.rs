pub mod log_settings;
pub mod menu;

#[cfg(windows)]
mod actions;
#[cfg(windows)]
mod entry;
#[cfg(windows)]
mod game_hooks;
#[cfg(windows)]
mod overlay;
#[cfg(windows)]
mod proxy;

pub const BUILD_COMMIT: &str = env!("NC_BUILD_COMMIT");
