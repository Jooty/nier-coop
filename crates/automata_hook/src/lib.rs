pub mod character_setting;
pub mod log_settings;
pub mod menu;
pub mod net;
pub mod start_setting;

#[cfg(windows)]
mod actions;
#[cfg(windows)]
mod boot;
#[cfg(windows)]
mod enemies_driver;
#[cfg(windows)]
mod entry;
#[cfg(windows)]
mod event_follow_driver;
#[cfg(windows)]
mod extra_bodies;
#[cfg(windows)]
mod game_calls;
#[cfg(windows)]
mod game_hooks;
#[cfg(windows)]
mod hud;
#[cfg(windows)]
mod overlay;
#[cfg(windows)]
mod partner_driver;
#[cfg(windows)]
mod pickups_driver;
#[cfg(windows)]
mod proxy;
#[cfg(windows)]
mod save_together_driver;
#[cfg(windows)]
mod shared_defeat_driver;
#[cfg(windows)]
mod start_driver;
#[cfg(windows)]
mod story_follow_driver;
#[cfg(windows)]
mod travel_driver;

pub const BUILD_COMMIT: &str = env!("NC_BUILD_COMMIT");
