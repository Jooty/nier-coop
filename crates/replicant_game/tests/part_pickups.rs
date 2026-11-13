use replicant_game::part_pickups::{
    PART_PICKUP_ERRANDS, PartnerPickupVerdict, PendingPartPickups, errand_of_handler,
    errand_of_state_step, local_pickup_to_send, partner_pickup_verdict,
};
use replicant_proto::PartPickup;

const ERRAND_HANDLER_RVA: u32 = 0x24_6890;
const ERRAND_STATE_STEP_RVA: u32 = 0x30_17A0;
const PICKUP_EVENT: u32 = 0x3C;
const ERRAND_PHASE: u16 = 7;

fn pickup(part_group: u16) -> PartPickup {
    PartPickup {
        phase: ERRAND_PHASE,
        actor_handler_rva: ERRAND_HANDLER_RVA,
        part_group,
    }
}

#[test]
fn the_errand_row_is_found_by_its_handler_and_by_its_counting_step() {
    let by_handler = errand_of_handler(ERRAND_HANDLER_RVA).unwrap();
    let by_step = errand_of_state_step(ERRAND_STATE_STEP_RVA).unwrap();
    assert_eq!(by_handler, by_step);
    assert_eq!(by_handler, &PART_PICKUP_ERRANDS[0]);
    assert!(errand_of_handler(0x24_6891).is_none());
}

#[test]
fn every_one_of_the_five_parts_the_actor_counts_is_a_pickup() {
    let errand = errand_of_handler(ERRAND_HANDLER_RVA).unwrap();
    for part_group in 10..=14 {
        assert!(errand.has_part_group(part_group));
    }
    assert!(!errand.has_part_group(9));
    assert!(!errand.has_part_group(15));
}

#[test]
fn a_local_pickup_of_a_counted_part_is_sent_with_this_zone() {
    let sent = local_pickup_to_send(
        ERRAND_HANDLER_RVA,
        PICKUP_EVENT,
        12,
        true,
        Some(ERRAND_PHASE),
    );
    assert_eq!(sent, Some(pickup(12)));
}

#[test]
fn other_events_parts_and_unlinked_games_send_nothing() {
    let other_event = local_pickup_to_send(ERRAND_HANDLER_RVA, 0x26, 12, true, Some(7));
    let other_part = local_pickup_to_send(ERRAND_HANDLER_RVA, PICKUP_EVENT, 3, true, Some(7));
    let alone = local_pickup_to_send(ERRAND_HANDLER_RVA, PICKUP_EVENT, 12, false, Some(7));
    let no_zone = local_pickup_to_send(ERRAND_HANDLER_RVA, PICKUP_EVENT, 12, true, None);
    let other_actor = local_pickup_to_send(0x24_0000, PICKUP_EVENT, 12, true, Some(7));
    assert_eq!(
        [other_event, other_part, alone, no_zone, other_actor],
        [None; 5]
    );
}

#[test]
fn a_partner_pickup_is_queued_only_in_the_same_zone() {
    assert_eq!(
        partner_pickup_verdict(&pickup(10), true, Some(ERRAND_PHASE)),
        PartnerPickupVerdict::Queue
    );
    assert_eq!(
        partner_pickup_verdict(&pickup(10), true, Some(8)),
        PartnerPickupVerdict::OtherPhase
    );
    assert_eq!(
        partner_pickup_verdict(&pickup(10), false, Some(ERRAND_PHASE)),
        PartnerPickupVerdict::NotConnected
    );
    assert_eq!(
        partner_pickup_verdict(&pickup(20), true, Some(ERRAND_PHASE)),
        PartnerPickupVerdict::PartOutsideErrand
    );
    let unknown = PartPickup {
        actor_handler_rva: 0x10,
        ..pickup(10)
    };
    assert_eq!(
        partner_pickup_verdict(&unknown, true, Some(ERRAND_PHASE)),
        PartnerPickupVerdict::UnknownErrand
    );
}

#[test]
fn partner_pickups_wait_until_the_actor_counts_its_parts() {
    let errand = errand_of_handler(ERRAND_HANDLER_RVA).unwrap();
    let mut pending = PendingPartPickups::new();
    assert!(pending.queue(pickup(10)));
    assert!(pending.queue(pickup(13)));
    assert!(
        pending
            .take_for_counting_step(errand, 0, Some(ERRAND_PHASE))
            .is_empty()
    );
    assert!(
        pending
            .take_for_counting_step(errand, 1, Some(ERRAND_PHASE))
            .is_empty()
    );
    assert_eq!(
        pending.take_for_counting_step(errand, 2, Some(ERRAND_PHASE)),
        vec![10, 13]
    );
    assert!(pending.is_empty());
}

#[test]
fn a_pickup_both_players_made_counts_once() {
    let errand = errand_of_handler(ERRAND_HANDLER_RVA).unwrap();
    let mut pending = PendingPartPickups::new();
    assert!(pending.queue(pickup(11)));
    assert!(!pending.queue(pickup(11)));
    assert_eq!(
        pending.take_for_counting_step(errand, 2, Some(ERRAND_PHASE)),
        vec![11]
    );
}

#[test]
fn pickups_of_another_zone_are_dropped_and_never_applied() {
    let errand = errand_of_handler(ERRAND_HANDLER_RVA).unwrap();
    let mut pending = PendingPartPickups::new();
    pending.queue(pickup(12));
    assert!(
        pending
            .take_for_counting_step(errand, 2, Some(ERRAND_PHASE + 1))
            .is_empty()
    );
    assert_eq!(pending.drop_other_phases(Some(ERRAND_PHASE + 1)), 1);
    assert_eq!(pending.len(), 0);
}
