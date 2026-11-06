use rc_game::Position;
use rc_game::arrival::{
    ARRIVAL_GIVE_UP_MS, ARRIVAL_SETTLE_MS, ArrivalGuard, ArrivalInputs, ArrivalStep,
    FALL_NO_GROUND_MS, FallGuard, FallInputs, PlaceReason,
};
use rc_game::stay_together::PhaseTarget;

const TARGET: PhaseTarget = PhaseTarget { phase: 4, point: 2 };

fn loaded(now_ms: u64) -> ArrivalInputs {
    ArrivalInputs {
        now_ms,
        local_phase: Some(4),
        partner_phase: Some(4),
        free_to_act: true,
        distance_to_partner: Some(30.0),
        ground_under_local: true,
    }
}

fn armed_guard() -> ArrivalGuard {
    let mut guard = ArrivalGuard::new();
    guard.arm(TARGET, 0);
    guard
}

fn falling(now_ms: u64, y: f32) -> FallInputs {
    FallInputs {
        now_ms,
        local_phase: Some(4),
        local: Position { x: 0.0, y, z: 0.0 },
        ground_under_local: false,
        partner_y_in_same_phase: Some(0.0),
    }
}

#[test]
fn an_unarmed_guard_does_nothing() {
    assert_eq!(ArrivalGuard::new().step(&loaded(5000)), ArrivalStep::Idle);
}

#[test]
fn the_guest_waits_while_the_walk_in_blocks_control() {
    let mut guard = armed_guard();
    let busy = ArrivalInputs {
        free_to_act: false,
        ..loaded(5000)
    };
    assert_eq!(guard.step(&busy), ArrivalStep::Waiting);
}

#[test]
fn the_guest_waits_until_the_host_reports_the_same_map() {
    let mut guard = armed_guard();
    let host_elsewhere = ArrivalInputs {
        partner_phase: Some(1),
        ..loaded(5000)
    };
    assert_eq!(guard.step(&host_elsewhere), ArrivalStep::Waiting);
}

#[test]
fn a_far_guest_is_placed_after_it_settles() {
    let mut guard = armed_guard();
    assert_eq!(guard.step(&loaded(5000)), ArrivalStep::Waiting);
    assert_eq!(
        guard.step(&loaded(5000 + ARRIVAL_SETTLE_MS)),
        ArrivalStep::Place(PlaceReason::TooFarFromHost)
    );
}

#[test]
fn a_guest_with_no_ground_is_placed_even_when_close() {
    let mut guard = armed_guard();
    let hovering = |now_ms| ArrivalInputs {
        distance_to_partner: Some(1.0),
        ground_under_local: false,
        ..loaded(now_ms)
    };
    guard.step(&hovering(5000));
    assert_eq!(
        guard.step(&hovering(5000 + ARRIVAL_SETTLE_MS)),
        ArrivalStep::Place(PlaceReason::NoGroundUnderYou)
    );
}

#[test]
fn a_guest_already_beside_the_host_is_left_alone() {
    let mut guard = armed_guard();
    let close = |now_ms| ArrivalInputs {
        distance_to_partner: Some(2.0),
        ..loaded(now_ms)
    };
    guard.step(&close(5000));
    assert_eq!(
        guard.step(&close(5000 + ARRIVAL_SETTLE_MS)),
        ArrivalStep::AlreadyBesideHost
    );
    assert!(!guard.is_armed());
}

#[test]
fn a_refused_placement_is_retried_until_it_is_confirmed() {
    let mut guard = armed_guard();
    let start = 5000 + ARRIVAL_SETTLE_MS;
    guard.step(&loaded(5000));
    assert!(matches!(guard.step(&loaded(start)), ArrivalStep::Place(_)));
    assert_eq!(guard.step(&loaded(start + 10)), ArrivalStep::Waiting);
    assert!(matches!(
        guard.step(&loaded(start + 2000)),
        ArrivalStep::Place(_)
    ));
    guard.placed();
    assert_eq!(guard.step(&loaded(start + 4000)), ArrivalStep::Idle);
}

#[test]
fn the_guard_gives_up_when_the_partner_never_shows() {
    let mut guard = armed_guard();
    let alone = ArrivalInputs {
        partner_phase: None,
        ..loaded(ARRIVAL_GIVE_UP_MS)
    };
    assert_eq!(guard.step(&alone), ArrivalStep::GaveUp);
    assert!(!guard.is_armed());
}

#[test]
fn standing_on_ground_never_counts_as_a_fall() {
    let mut guard = FallGuard::new();
    let standing = FallInputs {
        ground_under_local: true,
        ..falling(0, 0.0)
    };
    assert!(!guard.has_fallen(&standing));
}

#[test]
fn dropping_a_little_is_not_a_fall() {
    let mut guard = FallGuard::new();
    guard.has_fallen(&falling(0, -3.0));
    assert!(!guard.has_fallen(&falling(FALL_NO_GROUND_MS + 10, -3.0)));
}

#[test]
fn a_short_drop_through_the_air_is_not_a_fall() {
    let mut guard = FallGuard::new();
    assert!(!guard.has_fallen(&falling(0, -40.0)));
    assert!(!guard.has_fallen(&falling(FALL_NO_GROUND_MS - 100, -80.0)));
}

#[test]
fn long_and_deep_with_no_ground_is_a_fall_reported_once() {
    let mut guard = FallGuard::new();
    guard.has_fallen(&falling(0, -40.0));
    assert!(guard.has_fallen(&falling(FALL_NO_GROUND_MS, -90.0)));
    assert!(!guard.has_fallen(&falling(FALL_NO_GROUND_MS + 100, -120.0)));
}

#[test]
fn without_a_partner_the_last_grounded_height_is_the_reference() {
    let mut guard = FallGuard::new();
    let grounded = FallInputs {
        ground_under_local: true,
        partner_y_in_same_phase: None,
        ..falling(0, 10.0)
    };
    guard.has_fallen(&grounded);
    let lost = |now_ms, y| FallInputs {
        partner_y_in_same_phase: None,
        ..falling(now_ms, y)
    };
    guard.has_fallen(&lost(100, -20.0));
    assert!(guard.has_fallen(&lost(100 + FALL_NO_GROUND_MS, -60.0)));
    assert_eq!(guard.last_grounded().map(|spot| spot.y), Some(10.0));
}

#[test]
fn a_new_map_forgets_the_old_ground() {
    let mut guard = FallGuard::new();
    guard.has_fallen(&FallInputs {
        ground_under_local: true,
        ..falling(0, 10.0)
    });
    guard.has_fallen(&FallInputs {
        local_phase: Some(5),
        ..falling(10, 0.0)
    });
    assert_eq!(guard.last_grounded(), None);
}
