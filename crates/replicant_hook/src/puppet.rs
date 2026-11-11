mod actor;
mod body_pack;
mod book;
mod book_native;
mod error;
mod game_calls;
mod ghost;
pub(crate) mod pack_guard;
mod partner_body;
mod power;
mod power_send;
mod registry;
mod remote_driver;
mod ridden_boar;
mod weapon;
mod weapon_calls;

pub(crate) use book_native::install as install_book_native;
pub use ghost::{
    ghost_enabled, ghost_status, request_ghost_respawn, set_ghost_enabled, tick_ghost,
};
pub use power::{power_actor_addresses, tick_partner_powers};
pub use power_send::tick_own_powers;
pub use registry::{
    book_actor_address, book_actor_moved_to, despawn, first_actor_address, is_puppet_actor, tick,
};

use std::time::Duration;

const GHOST_DELAY_MS: u64 = 100;
const GHOST_PEER_TIMEOUT: Duration = Duration::from_secs(3);
const GHOST_RESPAWN_COOLDOWN: Duration = Duration::from_secs(1);
