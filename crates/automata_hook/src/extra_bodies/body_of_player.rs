use crate::extra_bodies::extra_bodies_driver::extra_body_of_slot;
use crate::net::{SessionModeExt, handle};
use crate::partner_driver::held_partner_body;

/// The body that stands for the player in the slot here: the companion slot body for the
/// companion player, the made body for every other player. Game thread only.
pub fn body_of_player(slot: u8) -> Option<usize> {
    let is_companion = handle()?.update(|shared| {
        shared
            .companion_player()
            .is_some_and(|player| player.slot == slot)
    });
    if is_companion {
        held_partner_body()
    } else {
        extra_body_of_slot(slot)
    }
}
