mod body_maker;
mod data_requests;
mod extra_bodies_driver;
mod session_players;

pub use extra_bodies_driver::{install, tick};

const EXTRA_BODY_LOG_OWNER_PREFIX: &str = "extra body of slot ";
