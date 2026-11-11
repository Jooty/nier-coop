use replicant_game::callback_gate::{ScenePull, ScenePullFacts, ScenePullSkip, scene_pull};
use replicant_game::story_calls::{PartnerPlace, SAME_PLACE_RANGE_UNITS};
use replicant_game::story_relay::{
    GatherAdmission, PULL_ARRIVAL_SETTLE_LIMIT_MS, PULL_ARRIVAL_SETTLE_MIN_MS, PullArrival,
    PullArrivalInputs, PullArrivalStep, SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS,
};

const FAR_UNITS: f32 = 46.8;
const CAME_FROM: [f32; 3] = [132.9, 40.4, 175.5];
const STARTER_AT: [f32; 3] = [86.8, 39.8, 164.1];
const BEGAN_MS: u64 = 10_000;

fn far_scene_start() -> ScenePullFacts {
    ScenePullFacts {
        starts_scene: true,
        led_by_partner: false,
        partner_place: PartnerPlace::TooFar { units: FAR_UNITS },
        admission: GatherAdmission::Open,
        relay_queue_has_room: true,
    }
}

#[test]
fn a_scene_start_with_the_partner_far_in_the_same_area_waits_and_asks_the_partner_over() {
    assert_eq!(
        scene_pull(&far_scene_start()),
        ScenePull::WaitAndAskPartnerOver { units: FAR_UNITS }
    );
}

#[test]
fn a_partner_who_is_near_is_not_waited_for() {
    let facts = ScenePullFacts {
        partner_place: PartnerPlace::Same,
        ..far_scene_start()
    };
    assert_eq!(
        scene_pull(&facts),
        ScenePull::RunNow(ScenePullSkip::PartnerHere)
    );
}

#[test]
fn a_partner_in_another_area_or_still_loading_is_not_waited_for() {
    let other_area = ScenePullFacts {
        partner_place: PartnerPlace::OtherPhase,
        ..far_scene_start()
    };
    assert_eq!(
        scene_pull(&other_area),
        ScenePull::RunNow(ScenePullSkip::PartnerInAnotherArea)
    );
    let loading = ScenePullFacts {
        partner_place: PartnerPlace::Unknown,
        ..far_scene_start()
    };
    assert_eq!(
        scene_pull(&loading),
        ScenePull::RunNow(ScenePullSkip::PartnerPlaceUnknown)
    );
}

#[test]
fn a_callback_that_starts_no_scene_never_waits() {
    let facts = ScenePullFacts {
        starts_scene: false,
        ..far_scene_start()
    };
    assert_eq!(
        scene_pull(&facts),
        ScenePull::RunNow(ScenePullSkip::NotASceneStart)
    );
}

#[test]
fn a_start_the_partner_led_never_waits() {
    let facts = ScenePullFacts {
        led_by_partner: true,
        ..far_scene_start()
    };
    assert_eq!(
        scene_pull(&facts),
        ScenePull::RunNow(ScenePullSkip::LedByPartner)
    );
}

#[test]
fn a_start_released_from_its_wait_runs_even_with_the_partner_still_far() {
    let facts = ScenePullFacts {
        admission: GatherAdmission::JustReleased,
        ..far_scene_start()
    };
    assert_eq!(
        scene_pull(&facts),
        ScenePull::RunNow(ScenePullSkip::ReleasedFromTheWait)
    );
}

#[test]
fn a_second_fire_of_a_start_that_already_waits_is_dropped() {
    let facts = ScenePullFacts {
        admission: GatherAdmission::AlreadyWaiting,
        ..far_scene_start()
    };
    assert_eq!(scene_pull(&facts), ScenePull::DropRepeatedFire);
}

#[test]
fn without_room_to_hold_it_the_start_runs_now() {
    let full = ScenePullFacts {
        admission: GatherAdmission::Full,
        ..far_scene_start()
    };
    assert_eq!(
        scene_pull(&full),
        ScenePull::RunNow(ScenePullSkip::NoRoomToWait)
    );
    let queue_full = ScenePullFacts {
        relay_queue_has_room: false,
        ..far_scene_start()
    };
    assert_eq!(
        scene_pull(&queue_full),
        ScenePull::RunNow(ScenePullSkip::NoRoomToWait)
    );
}

#[test]
fn only_the_skips_a_player_would_ask_about_are_logged() {
    for (skip, logged) in [
        (ScenePullSkip::NotASceneStart, false),
        (ScenePullSkip::LedByPartner, false),
        (ScenePullSkip::PartnerHere, false),
        (ScenePullSkip::ReleasedFromTheWait, false),
        (ScenePullSkip::PartnerInAnotherArea, true),
        (ScenePullSkip::PartnerPlaceUnknown, true),
        (ScenePullSkip::NoRoomToWait, true),
    ] {
        assert_eq!(skip.is_worth_a_log_line(), logged, "{skip:?}");
        assert!(!skip.describe().is_empty());
    }
}

fn pulled(starter_in_side_view: bool) -> PullArrival {
    let mut arrival = PullArrival::new();
    assert!(arrival.begin(CAME_FROM, STARTER_AT, starter_in_side_view, BEGAN_MS));
    arrival
}

fn at(waited_ms: u64, position: [f32; 3], in_side_view: bool) -> PullArrivalInputs {
    PullArrivalInputs {
        now_ms: BEGAN_MS + waited_ms,
        position: Some(position),
        in_side_view,
        door_steps_running: false,
    }
}

#[test]
fn a_pulled_player_is_ready_once_it_has_settled_beside_the_starter() {
    let mut arrival = pulled(false);
    assert_eq!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_MIN_MS - 1, STARTER_AT, false)),
        PullArrivalStep::Settling
    );
    assert_eq!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_MIN_MS, STARTER_AT, false)),
        PullArrivalStep::Ready {
            waited_ms: PULL_ARRIVAL_SETTLE_MIN_MS
        }
    );
    assert!(!arrival.is_settling());
    assert_eq!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_MIN_MS + 1, STARTER_AT, false)),
        PullArrivalStep::Idle
    );
}

#[test]
fn a_spot_within_the_same_place_range_counts_as_beside_the_starter() {
    let mut arrival = pulled(false);
    let near = [
        STARTER_AT[0] + SAME_PLACE_RANGE_UNITS - 1.0,
        STARTER_AT[1],
        STARTER_AT[2],
    ];
    assert!(matches!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_MIN_MS, near, false)),
        PullArrivalStep::Ready { .. }
    ));
}

#[test]
fn a_player_pulled_into_a_side_view_is_ready_only_when_its_own_game_turned_the_view_on() {
    let mut arrival = pulled(true);
    assert_eq!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_MIN_MS, STARTER_AT, false)),
        PullArrivalStep::Settling
    );
    assert_eq!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_MIN_MS + 100, STARTER_AT, true)),
        PullArrivalStep::Ready {
            waited_ms: PULL_ARRIVAL_SETTLE_MIN_MS + 100
        }
    );
}

#[test]
fn a_player_pulled_out_of_a_side_view_is_ready_only_when_its_own_game_turned_the_view_off() {
    let mut arrival = pulled(false);
    assert_eq!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_MIN_MS, STARTER_AT, true)),
        PullArrivalStep::Settling
    );
    assert!(matches!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_MIN_MS + 50, STARTER_AT, false)),
        PullArrivalStep::Ready { .. }
    ));
}

#[test]
fn a_player_whose_game_never_gives_the_view_goes_back_to_where_it_was() {
    let mut arrival = pulled(true);
    assert_eq!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_LIMIT_MS - 1, STARTER_AT, false)),
        PullArrivalStep::Settling
    );
    assert_eq!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_LIMIT_MS, STARTER_AT, false)),
        PullArrivalStep::GoBack {
            to: CAME_FROM,
            waited_ms: PULL_ARRIVAL_SETTLE_LIMIT_MS
        }
    );
    assert!(!arrival.is_settling());
}

#[test]
fn a_move_the_game_did_not_take_ends_without_a_move_back() {
    let mut arrival = pulled(false);
    assert_eq!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_LIMIT_MS, CAME_FROM, false)),
        PullArrivalStep::DidNotArrive {
            waited_ms: PULL_ARRIVAL_SETTLE_LIMIT_MS
        }
    );
    let mut unreadable = pulled(false);
    let step = unreadable.step(&PullArrivalInputs {
        now_ms: BEGAN_MS + PULL_ARRIVAL_SETTLE_LIMIT_MS,
        position: None,
        in_side_view: false,
        door_steps_running: false,
    });
    assert!(matches!(step, PullArrivalStep::DidNotArrive { .. }));
}

#[test]
fn a_second_pull_during_the_first_starts_no_second_move() {
    let mut arrival = pulled(false);
    assert!(!arrival.begin(STARTER_AT, CAME_FROM, true, BEGAN_MS + 10));
    assert!(matches!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_MIN_MS, STARTER_AT, false)),
        PullArrivalStep::Ready { .. }
    ));
}

#[test]
fn the_pulled_player_answers_before_the_starter_stops_waiting() {
    const { assert!(PULL_ARRIVAL_SETTLE_LIMIT_MS < SCENE_WAITS_FOR_THE_PULLED_PARTNER_MS) };
    const { assert!(PULL_ARRIVAL_SETTLE_MIN_MS < PULL_ARRIVAL_SETTLE_LIMIT_MS) };
}

#[test]
fn a_pulled_player_is_not_ready_while_a_door_step_is_still_to_run() {
    let mut arrival = pulled(true);
    let running = PullArrivalInputs {
        door_steps_running: true,
        ..at(PULL_ARRIVAL_SETTLE_MIN_MS, STARTER_AT, true)
    };
    assert_eq!(arrival.step(&running), PullArrivalStep::Settling);
    assert!(matches!(
        arrival.step(&at(PULL_ARRIVAL_SETTLE_MIN_MS + 50, STARTER_AT, true)),
        PullArrivalStep::Ready { .. }
    ));
}

#[test]
fn a_door_step_that_never_ends_does_not_send_a_player_with_the_right_view_back() {
    let mut arrival = pulled(true);
    let stuck = PullArrivalInputs {
        door_steps_running: true,
        ..at(PULL_ARRIVAL_SETTLE_LIMIT_MS, STARTER_AT, true)
    };
    assert!(matches!(
        arrival.step(&stuck),
        PullArrivalStep::Ready { .. }
    ));
}
