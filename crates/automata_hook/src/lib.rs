pub mod exe_facts;
pub mod log_settings;

#[cfg(windows)]
mod boot;
#[cfg(windows)]
mod entry;
#[cfg(windows)]
mod proxy;
