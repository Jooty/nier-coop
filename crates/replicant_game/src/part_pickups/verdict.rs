use replicant_proto::PartPickup;

use crate::part_pickups::errands::errand_of_handler;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartnerPickupVerdict {
    Queue,
    NotConnected,
    OtherPhase,
    UnknownErrand,
    PartOutsideErrand,
}

pub fn local_pickup_to_send(
    actor_handler_rva: u32,
    event: u32,
    part_group: u64,
    connected: bool,
    phase: Option<u16>,
) -> Option<PartPickup> {
    let errand = errand_of_handler(actor_handler_rva)?;
    if !connected || event != errand.pickup_event {
        return None;
    }
    let part_group = u16::try_from(part_group).ok()?;
    if !errand.has_part_group(part_group) {
        return None;
    }
    Some(PartPickup {
        phase: phase?,
        actor_handler_rva,
        part_group,
    })
}

pub fn partner_pickup_verdict(
    pickup: &PartPickup,
    connected: bool,
    local_phase: Option<u16>,
) -> PartnerPickupVerdict {
    if !connected {
        return PartnerPickupVerdict::NotConnected;
    }
    if local_phase != Some(pickup.phase) {
        return PartnerPickupVerdict::OtherPhase;
    }
    let Some(errand) = errand_of_handler(pickup.actor_handler_rva) else {
        return PartnerPickupVerdict::UnknownErrand;
    };
    if !errand.has_part_group(pickup.part_group) {
        return PartnerPickupVerdict::PartOutsideErrand;
    }
    PartnerPickupVerdict::Queue
}
