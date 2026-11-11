use replicant_game::Position;
use replicant_game::side_view::DoorMoveAction;
use replicant_game::stay_together::{
    ARRIVAL_PLACEMENT_RETRY_EVERY_MS, ARRIVAL_PLACEMENT_TRIES, HostViewFacts, HostViewReading,
    HostViewTracker, PhaseTarget, PlacementCheck, PlacementVerdict, ReadBack, decide_placement,
};

const OUTSIDE_THE_DOOR: Position = Position {
    x: 10.0,
    y: 2.0,
    z: -4.0,
};
const INSIDE_THE_HOUSE: Position = Position {
    x: 40.0,
    y: 2.0,
    z: 30.0,
};
const RETRY: u64 = ARRIVAL_PLACEMENT_RETRY_EVERY_MS;
const HOUSE_PHASE: u16 = 3;
const SCRIPTED_RETURN_POINT: i32 = 8;

fn host_in_normal_view() -> HostViewFacts {
    HostViewFacts {
        side_view_door: 0,
        in_side_view: false,
        last_normal_spot: Some(OUTSIDE_THE_DOOR),
    }
}

fn host_in_side_view(last_normal_spot: Option<Position>) -> HostViewFacts {
    HostViewFacts {
        side_view_door: 0,
        in_side_view: true,
        last_normal_spot,
    }
}

fn reading(point: i32, side_view: bool) -> HostViewReading {
    HostViewReading {
        phase: Some(HOUSE_PHASE),
        point,
        side_view,
        in_play: true,
        free_to_act: true,
        position: Some(if side_view {
            INSIDE_THE_HOUSE
        } else {
            OUTSIDE_THE_DOOR
        }),
    }
}

fn target(point: i32) -> PhaseTarget {
    PhaseTarget {
        phase: HOUSE_PHASE,
        point,
    }
}

#[test]
fn a_guest_places_beside_a_host_who_is_in_the_normal_view() {
    assert_eq!(
        decide_placement(Some(host_in_normal_view()), false, DoorMoveAction::Nothing),
        PlacementVerdict::BesideHost
    );
}

#[test]
fn a_guest_without_any_host_facts_places_beside_the_host() {
    assert_eq!(
        decide_placement(None, false, DoorMoveAction::Nothing),
        PlacementVerdict::BesideHost
    );
}

#[test]
fn a_guest_outside_goes_to_the_hosts_last_spot_outside_when_the_host_is_in_a_side_view() {
    assert_eq!(
        decide_placement(
            Some(host_in_side_view(Some(OUTSIDE_THE_DOOR))),
            false,
            DoorMoveAction::Nothing
        ),
        PlacementVerdict::OutsideSpot(OUTSIDE_THE_DOOR)
    );
}

#[test]
fn a_guest_stays_at_the_entry_point_when_no_spot_outside_is_known() {
    assert_eq!(
        decide_placement(
            Some(host_in_side_view(None)),
            false,
            DoorMoveAction::Nothing
        ),
        PlacementVerdict::StayAtEntry
    );
}

#[test]
fn a_guest_in_a_side_view_stays_put_when_the_host_is_in_the_normal_view() {
    assert_eq!(
        decide_placement(Some(host_in_normal_view()), true, DoorMoveAction::Nothing),
        PlacementVerdict::StayWhereYouAre
    );
}

#[test]
fn a_guest_already_in_a_side_view_places_beside_a_host_in_a_side_view() {
    assert_eq!(
        decide_placement(
            Some(host_in_side_view(Some(OUTSIDE_THE_DOOR))),
            true,
            DoorMoveAction::Nothing
        ),
        PlacementVerdict::BesideHost
    );
}

#[test]
fn a_point_that_put_the_host_in_a_side_view_on_arrival_is_announced_as_point_zero() {
    let mut tracker = HostViewTracker::new();
    assert_eq!(tracker.announced_point(target(SCRIPTED_RETURN_POINT)), 8);
    let marked = tracker.observe(&reading(SCRIPTED_RETURN_POINT, true));
    assert_eq!(marked, Some(SCRIPTED_RETURN_POINT));
    assert_eq!(tracker.announced_point(target(SCRIPTED_RETURN_POINT)), 0);
}

#[test]
fn only_the_marked_point_in_the_marked_phase_is_replaced() {
    let mut tracker = HostViewTracker::new();
    tracker.observe(&reading(SCRIPTED_RETURN_POINT, true));
    assert_eq!(tracker.announced_point(target(2)), 2);
    let other_phase = PhaseTarget {
        phase: HOUSE_PHASE + 1,
        point: SCRIPTED_RETURN_POINT,
    };
    assert_eq!(tracker.announced_point(other_phase), SCRIPTED_RETURN_POINT);
}

#[test]
fn a_point_is_marked_once() {
    let mut tracker = HostViewTracker::new();
    assert!(
        tracker
            .observe(&reading(SCRIPTED_RETURN_POINT, true))
            .is_some()
    );
    assert_eq!(tracker.observe(&reading(SCRIPTED_RETURN_POINT, true)), None);
}

#[test]
fn a_host_in_the_normal_view_when_the_arrival_settles_marks_nothing() {
    let mut tracker = HostViewTracker::new();
    assert_eq!(tracker.observe(&reading(5, false)), None);
    assert_eq!(tracker.announced_point(target(5)), 5);
}

#[test]
fn walking_into_a_side_view_after_the_arrival_settled_marks_nothing() {
    let mut tracker = HostViewTracker::new();
    tracker.observe(&reading(5, false));
    assert_eq!(tracker.observe(&reading(5, true)), None);
    assert_eq!(tracker.announced_point(target(5)), 5);
}

#[test]
fn the_arrival_settles_only_once_the_host_is_in_play_with_controls_free() {
    let mut tracker = HostViewTracker::new();
    let busy = HostViewReading {
        free_to_act: false,
        ..reading(SCRIPTED_RETURN_POINT, true)
    };
    let loading = HostViewReading {
        in_play: false,
        ..reading(SCRIPTED_RETURN_POINT, true)
    };
    assert_eq!(tracker.observe(&busy), None);
    assert_eq!(tracker.observe(&loading), None);
    assert_eq!(tracker.announced_point(target(SCRIPTED_RETURN_POINT)), 8);
    assert_eq!(
        tracker.observe(&reading(SCRIPTED_RETURN_POINT, true)),
        Some(SCRIPTED_RETURN_POINT)
    );
}

#[test]
fn a_reload_of_the_same_point_settles_again() {
    let mut tracker = HostViewTracker::new();
    tracker.observe(&reading(5, false));
    let unreadable = HostViewReading {
        phase: None,
        ..reading(5, false)
    };
    tracker.observe(&unreadable);
    assert_eq!(tracker.observe(&reading(5, true)), Some(5));
}

#[test]
fn point_zero_is_never_marked() {
    let mut tracker = HostViewTracker::new();
    assert_eq!(tracker.observe(&reading(0, true)), None);
    assert_eq!(tracker.announced_point(target(0)), 0);
}

#[test]
fn the_last_normal_spot_follows_the_host_outside_a_side_view_only() {
    let mut tracker = HostViewTracker::new();
    assert_eq!(tracker.facts(false, 0).last_normal_spot, None);
    tracker.observe(&reading(5, false));
    assert_eq!(
        tracker.facts(false, 0).last_normal_spot,
        Some(OUTSIDE_THE_DOOR)
    );
    tracker.observe(&reading(5, true));
    let facts = tracker.facts(true, 0);
    assert!(facts.in_side_view);
    assert_eq!(facts.last_normal_spot, Some(OUTSIDE_THE_DOOR));
}

#[test]
fn the_last_normal_spot_is_forgotten_on_a_phase_change() {
    let mut tracker = HostViewTracker::new();
    tracker.observe(&reading(5, false));
    let next_phase = HostViewReading {
        phase: Some(HOUSE_PHASE + 1),
        ..reading(1, true)
    };
    tracker.observe(&next_phase);
    assert_eq!(tracker.facts(true, 0).last_normal_spot, None);
}

#[test]
fn a_host_that_is_not_in_play_leaves_the_last_normal_spot_alone() {
    let mut tracker = HostViewTracker::new();
    tracker.observe(&reading(5, false));
    let loading = HostViewReading {
        in_play: false,
        position: Some(Position::default()),
        ..reading(5, false)
    };
    tracker.observe(&loading);
    assert_eq!(
        tracker.facts(false, 0).last_normal_spot,
        Some(OUTSIDE_THE_DOOR)
    );
}

#[test]
fn a_placement_the_game_took_is_confirmed_by_the_readback() {
    let mut check = PlacementCheck::requested(OUTSIDE_THE_DOOR, 0);
    let nearby = Position {
        x: OUTSIDE_THE_DOOR.x + 0.4,
        ..OUTSIDE_THE_DOOR
    };
    assert_eq!(check.read_back(nearby, 16), ReadBack::Placed);
}

#[test]
fn a_placement_the_game_ignored_is_retried_and_counted() {
    let mut check = PlacementCheck::requested(OUTSIDE_THE_DOOR, 0);
    assert_eq!(
        check.read_back(INSIDE_THE_HOUSE, RETRY),
        ReadBack::NotTaken { try_number: 1 }
    );
    assert_eq!(
        check.read_back(INSIDE_THE_HOUSE, RETRY * 2),
        ReadBack::NotTaken { try_number: 2 }
    );
    assert_eq!(check.tries(), 3);
    assert_eq!(
        check.read_back(OUTSIDE_THE_DOOR, RETRY * 2),
        ReadBack::Placed
    );
}

#[test]
fn no_second_request_comes_before_the_retry_interval() {
    let mut check = PlacementCheck::requested(OUTSIDE_THE_DOOR, 1000);
    for now_ms in [1000, 1016, 1100, 1000 + RETRY - 1] {
        assert_eq!(check.read_back(INSIDE_THE_HOUSE, now_ms), ReadBack::Waiting);
    }
    assert_eq!(check.tries(), 1);
    assert_eq!(
        check.read_back(INSIDE_THE_HOUSE, 1000 + RETRY),
        ReadBack::NotTaken { try_number: 1 }
    );
    assert_eq!(
        check.read_back(INSIDE_THE_HOUSE, 1000 + RETRY + 100),
        ReadBack::Waiting
    );
}

#[test]
fn a_readback_success_between_requests_ends_the_retries() {
    let mut check = PlacementCheck::requested(OUTSIDE_THE_DOOR, 0);
    assert_eq!(check.read_back(INSIDE_THE_HOUSE, 100), ReadBack::Waiting);
    assert_eq!(check.read_back(OUTSIDE_THE_DOOR, 120), ReadBack::Placed);
}

#[test]
fn the_placement_gives_up_after_the_allowed_tries() {
    let mut check = PlacementCheck::requested(OUTSIDE_THE_DOOR, 0);
    for request in 1..ARRIVAL_PLACEMENT_TRIES {
        assert_eq!(
            check.read_back(INSIDE_THE_HOUSE, RETRY * u64::from(request)),
            ReadBack::NotTaken {
                try_number: request
            }
        );
    }
    let last_request_due_ms = RETRY * u64::from(ARRIVAL_PLACEMENT_TRIES);
    assert_eq!(
        check.read_back(INSIDE_THE_HOUSE, last_request_due_ms),
        ReadBack::GaveUp {
            tries: ARRIVAL_PLACEMENT_TRIES
        }
    );
}

#[test]
fn a_spot_just_past_the_arrived_distance_is_not_taken() {
    let mut check = PlacementCheck::requested(OUTSIDE_THE_DOOR, 0);
    let off = Position {
        x: OUTSIDE_THE_DOOR.x + 1.5,
        ..OUTSIDE_THE_DOOR
    };
    assert_eq!(
        check.read_back(off, RETRY),
        ReadBack::NotTaken { try_number: 1 }
    );
}

const DOOR_STEPS: DoorMoveAction = DoorMoveAction::RunSteps(&["DoorStepOne", "DoorStepTwo"]);

#[test]
fn a_guest_goes_beside_a_host_across_a_door_when_the_door_steps_may_run() {
    assert_eq!(
        decide_placement(
            Some(host_in_side_view(Some(OUTSIDE_THE_DOOR))),
            false,
            DOOR_STEPS
        ),
        PlacementVerdict::BesideHostThroughDoor(DOOR_STEPS)
    );
    assert_eq!(
        decide_placement(Some(host_in_normal_view()), true, DOOR_STEPS),
        PlacementVerdict::BesideHostThroughDoor(DOOR_STEPS)
    );
}

#[test]
fn door_steps_are_not_used_when_both_players_share_a_view() {
    assert_eq!(
        decide_placement(Some(host_in_side_view(None)), true, DOOR_STEPS),
        PlacementVerdict::BesideHost
    );
    assert_eq!(
        decide_placement(Some(host_in_normal_view()), false, DOOR_STEPS),
        PlacementVerdict::BesideHost
    );
}
