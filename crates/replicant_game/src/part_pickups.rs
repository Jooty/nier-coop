mod errands;
mod pending;
mod verdict;

pub use errands::{PART_PICKUP_ERRANDS, PartPickupErrand, errand_of_handler, errand_of_state_step};
pub use pending::PendingPartPickups;
pub use verdict::{PartnerPickupVerdict, local_pickup_to_send, partner_pickup_verdict};

pub const PART_VISIBILITY_FUNCTION_RVA: u32 = 0x15_A050;
pub const MAX_PENDING_PART_PICKUPS: usize = 32;
