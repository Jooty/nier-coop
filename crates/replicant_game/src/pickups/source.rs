use replicant_proto::PickupSource;

use crate::offsets::{PLAYER_MESSAGE_HANDLER_RVA_RANGE, SHOP_TRANSACTION_RVA_RANGE};

pub fn classify_add_caller(caller_rva: u32) -> PickupSource {
    if within(caller_rva, PLAYER_MESSAGE_HANDLER_RVA_RANGE) {
        PickupSource::World
    } else if within(caller_rva, SHOP_TRANSACTION_RVA_RANGE) {
        PickupSource::Shop
    } else {
        PickupSource::Other
    }
}

fn within(rva: u32, (first, last): (u32, u32)) -> bool {
    (first..=last).contains(&rva)
}
