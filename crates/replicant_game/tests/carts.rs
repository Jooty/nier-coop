use replicant_game::Position;
use replicant_game::carts::{
    CART_BOARD_GIVE_UP_MS, CART_BOARD_SETTLE_MS, CART_END_GRACE_MS, CART_END_PULL_DELAY_MS,
    CART_MATCH_UNITS, CART_PARTNER_NEAR_UNITS, CART_RIDER_LIFT_UNITS, CART_STATE_OFFSET,
    CART_VTABLE_RVA, CartFollowStep, CartRideEvent, CartRideFacts, CartSighting, CartState,
    GuestCartFollow, HostRideWatch, OwnCartFacts, cart_follow_step_text, cart_state, match_cart,
    parked_state_after, run_state_from, scan_cart_actors,
};
use replicant_game::offsets::{
    ACTOR_LIFE_STATE_OFFSET, ACTOR_MANAGER_RVA, ACTOR_MANAGER_SECONDARY_LIST_HEAD_OFFSET,
    ACTOR_SECONDARY_LIST_NEXT_OFFSET, ACTOR_SELF_HANDLE_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES,
    ACTOR_TRANSFORM_BLOCK_OFFSET, HANDLE_POOL_GENERATIONS_RVA, HANDLE_POOL_POINTERS_RVA,
};
use replicant_game::puppet::PuppetPose;
use replicant_game::stay_together::{JumpPartner, JumpStep};
use replicant_proto::CartRide;

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const PHASE: u16 = 13;
const CART: u64 = 0x5000_0000;
const OTHER_CART: u64 = 0x5100_0000;
const GUEST_CART: u64 = 0x6000_0000;
const PARKED_AT: [f32; 3] = [10.0, 0.0, -105.0];
const RIDER_OFFSET: [f32; 3] = [0.5, 1.0, 0.0];

fn cart(actor: u64, state: CartState, position: [f32; 3]) -> CartSighting {
    CartSighting {
        actor,
        layout_id: Some(41),
        state,
        position,
    }
}

fn plus(position: [f32; 3], offset: [f32; 3]) -> [f32; 3] {
    [
        position[0] + offset[0],
        position[1] + offset[1],
        position[2] + offset[2],
    ]
}

fn host_facts(now_ms: u64, carts: &[CartSighting], rider_on_cart: bool) -> CartRideFacts<'_> {
    CartRideFacts {
        now_ms,
        phase: Some(PHASE),
        carts,
        rider_on_cart,
        rider_position: carts.first().map(|cart| plus(cart.position, RIDER_OFFSET)),
    }
}

fn started_ride() -> CartRide {
    CartRide {
        phase: PHASE,
        layout_id: Some(41),
        cart_position: PARKED_AT,
        from_state: 1,
        to_state: 5,
        rider_offset: RIDER_OFFSET,
    }
}

fn ended_ride() -> CartRide {
    CartRide {
        phase: PHASE,
        layout_id: Some(41),
        cart_position: [-37.0, -0.5, -42.0],
        from_state: 5,
        to_state: 2,
        rider_offset: RIDER_OFFSET,
    }
}

fn own_facts(now_ms: u64, carts: &[CartSighting], may_place_rider: bool) -> OwnCartFacts<'_> {
    OwnCartFacts {
        now_ms,
        phase: Some(PHASE),
        carts,
        may_place_rider,
    }
}

fn put_cart(memory: &mut FakeMemory, actor: u64, slot: u32, next: u64, state: i32) {
    memory.put(
        actor,
        &(FAKE_MODULE_BASE + u64::from(CART_VTABLE_RVA)).to_le_bytes(),
    );
    memory.put(
        actor + ACTOR_SECONDARY_LIST_NEXT_OFFSET,
        &next.to_le_bytes(),
    );
    let mut handle = [0_u8; 8];
    handle[..4].copy_from_slice(&slot.to_le_bytes());
    handle[4..].copy_from_slice(&1_u32.to_le_bytes());
    memory.put(actor + ACTOR_SELF_HANDLE_OFFSET, &handle);
    memory.put_at_rva(HANDLE_POOL_POINTERS_RVA + slot * 8, &actor.to_le_bytes());
    memory.put_at_rva(HANDLE_POOL_GENERATIONS_RVA + slot * 4, &1_u32.to_le_bytes());
    memory.put(actor + ACTOR_LIFE_STATE_OFFSET, &0_i32.to_le_bytes());
    memory.put(actor + CART_STATE_OFFSET, &state.to_le_bytes());
    let pose = PuppetPose {
        position: Position {
            x: 10.0,
            y: 0.0,
            z: -105.0,
        },
        yaw: 0.0,
    };
    memory.put(
        actor + ACTOR_TRANSFORM_BLOCK_OFFSET,
        &pose.creation_matrix()[..ACTOR_TRANSFORM_BLOCK_BYTES],
    );
}

#[test]
fn boarding_runs_the_parked_end_and_a_run_parks_at_the_other_end() {
    assert_eq!(run_state_from(1), Some(5));
    assert_eq!(run_state_from(4), Some(8));
    assert_eq!(run_state_from(5), None);
    assert_eq!(parked_state_after(5), Some(2));
    assert_eq!(parked_state_after(6), Some(1));
    assert_eq!(parked_state_after(7), Some(4));
    assert_eq!(parked_state_after(8), Some(3));
    assert_eq!(parked_state_after(2), None);
    assert_eq!(cart_state(3), CartState::Parked(3));
    assert_eq!(cart_state(7), CartState::Running(7));
    assert_eq!(cart_state(0), CartState::Unset);
    assert_eq!(cart_state(200), CartState::Unset);
}

#[test]
fn the_scan_lists_live_carts_on_the_actor_list_with_their_state_and_place() {
    let mut memory = FakeMemory::new();
    let head =
        u32::try_from(u64::from(ACTOR_MANAGER_RVA) + ACTOR_MANAGER_SECONDARY_LIST_HEAD_OFFSET)
            .expect("rva");
    memory.put_at_rva(head, &CART.to_le_bytes());
    put_cart(&mut memory, CART, 1, OTHER_CART, 3);
    put_cart(&mut memory, OTHER_CART, 2, 0, 6);
    memory.put(OTHER_CART, &0x1234_u64.to_le_bytes());
    let found = scan_cart_actors(&memory, |actor| (actor == CART).then_some(7));
    assert_eq!(
        found,
        vec![CartSighting {
            actor: CART,
            layout_id: Some(7),
            state: CartState::Parked(3),
            position: PARKED_AT,
        }]
    );
}

#[test]
fn a_report_finds_the_cart_by_layout_id_first_then_by_place() {
    let near = cart(CART, CartState::Parked(1), [12.0, 0.0, -105.0]);
    let far = CartSighting {
        layout_id: Some(9),
        ..cart(OTHER_CART, CartState::Parked(2), [90.0, 0.0, 0.0])
    };
    assert_eq!(
        match_cart(&[near, far], Some(9), PARKED_AT).map(|c| c.actor),
        Some(OTHER_CART)
    );
    assert_eq!(
        match_cart(&[near, far], Some(77), PARKED_AT).map(|c| c.actor),
        Some(CART)
    );
    let too_far = cart(
        CART,
        CartState::Parked(1),
        plus(PARKED_AT, [CART_MATCH_UNITS + 1.0, 0.0, 0.0]),
    );
    assert_eq!(match_cart(&[too_far], None, PARKED_AT), None);
}

#[test]
fn the_host_reports_a_ride_it_boards_from_the_parked_place() {
    let mut watch = HostRideWatch::new();
    let parked = [cart(CART, CartState::Parked(1), PARKED_AT)];
    assert!(watch.observe(&host_facts(0, &parked, false)).is_empty());
    assert!(watch.observe(&host_facts(100, &parked, false)).is_empty());
    let moving = [cart(
        CART,
        CartState::Running(5),
        plus(PARKED_AT, [0.0, 0.0, 0.2]),
    )];
    let events = watch.observe(&host_facts(200, &moving, true));
    let [CartRideEvent::Started(ride)] = events.as_slice() else {
        panic!("one start, got {events:?}");
    };
    assert_eq!(ride.cart_position, PARKED_AT);
    assert_eq!((ride.from_state, ride.to_state), (1, 5));
    assert!((ride.rider_offset[1] - RIDER_OFFSET[1]).abs() < 1e-4);
    assert!(watch.riding());
}

#[test]
fn a_cart_that_runs_without_the_host_on_it_is_not_a_ride() {
    let mut watch = HostRideWatch::new();
    let parked = [cart(CART, CartState::Parked(1), PARKED_AT)];
    watch.observe(&host_facts(0, &parked, false));
    let moving = [cart(CART, CartState::Running(5), PARKED_AT)];
    assert!(watch.observe(&host_facts(100, &moving, false)).is_empty());
    let back = [cart(CART, CartState::Parked(2), PARKED_AT)];
    assert!(watch.observe(&host_facts(200, &back, false)).is_empty());
}

#[test]
fn the_ride_end_is_reported_and_a_far_partner_is_brought_a_moment_later() {
    let mut watch = HostRideWatch::new();
    let end = [-37.0, -0.5, -42.0];
    watch.observe(&host_facts(
        0,
        &[cart(CART, CartState::Parked(1), PARKED_AT)],
        false,
    ));
    watch.observe(&host_facts(
        100,
        &[cart(CART, CartState::Running(5), PARKED_AT)],
        true,
    ));
    let events = watch.observe(&host_facts(
        30_000,
        &[cart(CART, CartState::Parked(2), end)],
        false,
    ));
    let [CartRideEvent::Ended(ride)] = events.as_slice() else {
        panic!("one end, got {events:?}");
    };
    assert_eq!((ride.from_state, ride.to_state), (5, 2));
    assert_eq!(ride.cart_position, end);
    let far = Some(JumpPartner {
        position: PARKED_AT,
        phase: PHASE,
    });
    assert_eq!(watch.pull_due(30_000, Some(end), far), JumpStep::Nothing);
    let due = 30_000 + CART_END_PULL_DELAY_MS;
    let JumpStep::BringPartner { from, to, .. } = watch.pull_due(due, Some(end), far) else {
        panic!("the far partner is brought");
    };
    assert_eq!((from, to), (PARKED_AT, end));
    assert_eq!(watch.pull_due(due + 1, Some(end), far), JumpStep::Nothing);
}

#[test]
fn a_partner_who_rode_along_or_is_in_another_map_is_left_where_it_is() {
    let end = [-37.0, -0.5, -42.0];
    let ride_once = |watch: &mut HostRideWatch| {
        watch.observe(&host_facts(
            0,
            &[cart(CART, CartState::Parked(1), PARKED_AT)],
            false,
        ));
        watch.observe(&host_facts(
            100,
            &[cart(CART, CartState::Running(5), PARKED_AT)],
            true,
        ));
        watch.observe(&host_facts(
            200,
            &[cart(CART, CartState::Parked(2), end)],
            false,
        ));
    };
    let due = 200 + CART_END_PULL_DELAY_MS;
    let mut watch = HostRideWatch::new();
    ride_once(&mut watch);
    let beside = Some(JumpPartner {
        position: plus(end, [CART_PARTNER_NEAR_UNITS - 1.0, 0.0, 0.0]),
        phase: PHASE,
    });
    assert_eq!(watch.pull_due(due, Some(end), beside), JumpStep::Nothing);
    let mut watch = HostRideWatch::new();
    ride_once(&mut watch);
    let elsewhere = Some(JumpPartner {
        position: PARKED_AT,
        phase: PHASE + 1,
    });
    assert_eq!(watch.pull_due(due, Some(end), elsewhere), JumpStep::Nothing);
}

#[test]
fn a_map_change_forgets_what_the_host_saw() {
    let mut watch = HostRideWatch::new();
    watch.observe(&host_facts(
        0,
        &[cart(CART, CartState::Parked(1), PARKED_AT)],
        false,
    ));
    let mut other_map = host_facts(100, &[], false);
    other_map.phase = Some(PHASE + 1);
    watch.observe(&other_map);
    let running = [cart(CART, CartState::Running(5), PARKED_AT)];
    assert!(watch.observe(&host_facts(200, &running, true)).is_empty());
}

#[test]
fn the_guest_stands_on_its_own_cart_and_starts_it_after_a_moment() {
    let mut follow = GuestCartFollow::new();
    follow.host_rode(started_ride(), 0);
    let own = [cart(GUEST_CART, CartState::Parked(1), PARKED_AT)];
    let CartFollowStep::PlaceRider { actor, spot } = follow.step(&own_facts(0, &own, true)) else {
        panic!("the rider is placed first");
    };
    assert_eq!(actor, GUEST_CART);
    let wanted = plus(
        plus(PARKED_AT, RIDER_OFFSET),
        [0.0, CART_RIDER_LIFT_UNITS, 0.0],
    );
    assert!(spot.iter().zip(wanted).all(|(a, b)| (a - b).abs() < 1e-4));
    assert_eq!(
        follow.step(&own_facts(CART_BOARD_SETTLE_MS - 1, &own, true)),
        CartFollowStep::Nothing
    );
    assert_eq!(
        follow.step(&own_facts(CART_BOARD_SETTLE_MS, &own, true)),
        CartFollowStep::Start {
            actor: GUEST_CART,
            state: 5
        }
    );
    assert_eq!(follow.riding_actor(), Some(GUEST_CART));
}

#[test]
fn a_guest_cart_at_the_other_end_is_brought_to_the_host_start_first() {
    let mut follow = GuestCartFollow::new();
    follow.host_rode(started_ride(), 0);
    let wrong_end = [cart(GUEST_CART, CartState::Parked(2), [-37.0, -0.5, -42.0])];
    assert_eq!(
        follow.step(&own_facts(0, &wrong_end, true)),
        CartFollowStep::Park {
            actor: GUEST_CART,
            state: 1
        }
    );
    let parked = [cart(GUEST_CART, CartState::Parked(1), PARKED_AT)];
    assert!(matches!(
        follow.step(&own_facts(100, &parked, true)),
        CartFollowStep::PlaceRider { .. }
    ));
}

#[test]
fn a_guest_in_a_talk_waits_and_then_gives_the_ride_up_to_the_end_pull() {
    let mut follow = GuestCartFollow::new();
    follow.host_rode(started_ride(), 0);
    let own = [cart(GUEST_CART, CartState::Parked(1), PARKED_AT)];
    assert_eq!(
        follow.step(&own_facts(100, &own, false)),
        CartFollowStep::Nothing
    );
    let step = follow.step(&own_facts(CART_BOARD_GIVE_UP_MS, &own, false));
    assert!(matches!(step, CartFollowStep::GaveUpBoarding { .. }));
    assert!(cart_follow_step_text(&step).is_some());
    assert_eq!(
        follow.step(&own_facts(CART_BOARD_GIVE_UP_MS + 100, &own, true)),
        CartFollowStep::Nothing
    );
}

#[test]
fn a_guest_without_a_matching_cart_or_in_another_map_is_left_to_the_end_pull() {
    let mut follow = GuestCartFollow::new();
    follow.host_rode(started_ride(), 0);
    let step = follow.step(&own_facts(0, &[], true));
    assert!(matches!(step, CartFollowStep::NoCartHere { .. }));
    let mut follow = GuestCartFollow::new();
    follow.host_rode(started_ride(), 0);
    let own = [cart(GUEST_CART, CartState::Parked(1), PARKED_AT)];
    let mut elsewhere = own_facts(0, &own, true);
    elsewhere.phase = Some(PHASE + 1);
    assert_eq!(follow.step(&elsewhere), CartFollowStep::OtherMap);
}

#[test]
fn a_guest_cart_still_running_after_the_host_ride_ended_is_put_at_that_end() {
    let mut follow = GuestCartFollow::new();
    follow.host_rode(started_ride(), 0);
    let own = [cart(GUEST_CART, CartState::Parked(1), PARKED_AT)];
    follow.step(&own_facts(0, &own, true));
    follow.step(&own_facts(CART_BOARD_SETTLE_MS, &own, true));
    let running = [cart(GUEST_CART, CartState::Running(5), [0.0, 0.0, -60.0])];
    follow.step(&own_facts(10_000, &running, true));
    follow.host_rode(ended_ride(), 20_000);
    assert_eq!(
        follow.step(&own_facts(20_000 + CART_END_GRACE_MS - 1, &running, true)),
        CartFollowStep::Nothing
    );
    assert_eq!(
        follow.step(&own_facts(20_000 + CART_END_GRACE_MS, &running, true)),
        CartFollowStep::ParkAtEnd {
            actor: GUEST_CART,
            state: 2
        }
    );
}

#[test]
fn a_guest_cart_that_reached_the_same_end_is_left_alone() {
    let mut follow = GuestCartFollow::new();
    follow.host_rode(ended_ride(), 0);
    let arrived = [cart(GUEST_CART, CartState::Parked(2), [-37.0, -0.5, -42.0])];
    assert_eq!(
        follow.step(&own_facts(CART_END_GRACE_MS, &arrived, true)),
        CartFollowStep::Nothing
    );
}
