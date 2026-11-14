use automata_game::partner::{POSITION_ERROR_TELEPORTED, STATES_STARTED_ON_THE_BODY};
use automata_game::partner_flight::{
    BodyFlight, BoundedTries, FlightCall, FlightFields, FlightKind, FlightMode, FlightReadings,
    FlightReport, FlightSteering, PARTNER_BOARD_RETRY_MS, PARTNER_BOARD_TRIES,
    PARTNER_GET_OFF_TRIES, PARTNER_MODE_TRIES, PARTNER_OUT_OF_A_UNIT_BEFORE_THE_BODY_GETS_OFF_MS,
    PartnerFlight, RIDING_MISMATCH_LOGGED_AFTER_MS, StickChoice, TryVerdict, UnitPlacer, UnitPose,
    flight_report_of, flight_stick_in_use, gets_off_before_release, is_a_flight_state,
    is_a_shooting_stage_state,
};

const SHOOTING_IDLE_STATE: i32 = 0x84;
const FREE_FLIGHT_IDLE_STATE: i32 = 0x70;
const ON_FOOT_IDLE_STATE: i32 = 0;

fn partner(kind: FlightKind, mode: FlightMode) -> FlightReport {
    FlightReport {
        riding: kind == FlightKind::FreeFlight,
        kind,
        mode,
        unit: kind.is_in_a_unit().then_some(UnitPose {
            position: [1.0, 2.0, 3.0],
            yaw: 0.5,
        }),
        stick: kind.is_in_a_unit().then_some([0.25, -0.75]),
    }
}

fn on_foot() -> FlightReport {
    FlightReport::default()
}

fn body(riding: bool, mode: FlightMode, state: i32) -> BodyFlight {
    BodyFlight {
        riding,
        mode,
        state,
        alive: true,
        shooting_stage: false,
        unit_found: riding,
        steering: FlightSteering::Player,
    }
}

fn readings(partner: FlightReport, body: BodyFlight) -> FlightReadings {
    FlightReadings {
        applies: true,
        partner: Some(partner),
        body: Some(body),
        hero_in_a_unit: partner.is_in_a_unit(),
    }
}

fn boarding() -> FlightReadings {
    readings(
        partner(FlightKind::ShootingStage, FlightMode::Robot),
        body(false, FlightMode::Robot, ON_FOOT_IDLE_STATE),
    )
}

#[test]
fn the_body_boards_when_the_partners_hero_is_in_a_unit_and_the_body_is_not() {
    let mut flight = PartnerFlight::default();
    let step = flight.step(0, &boarding());
    assert_eq!(step.call, Some(FlightCall::Board));
    assert!(step.logs[0].contains("startPhaseEventFlightUnit"));
    assert!(step.logs[0].contains("try 1 of 5"));
}

#[test]
fn a_board_is_retried_once_a_second_at_most_five_times_then_refused() {
    let mut flight = PartnerFlight::default();
    let mut boards = 0;
    let mut refusals = 0;
    for now_ms in (0..20_000).step_by(100) {
        let step = flight.step(now_ms, &boarding());
        if step.call == Some(FlightCall::Board) {
            boards += 1;
            assert_eq!(now_ms % PARTNER_BOARD_RETRY_MS, 0);
        }
        refusals += step
            .logs
            .iter()
            .filter(|line| line.contains("did not get into a flight unit after 5 tries"))
            .count();
    }
    assert_eq!(boards, PARTNER_BOARD_TRIES);
    assert_eq!(refusals, 1);
}

#[test]
fn a_refused_board_is_tried_again_once_the_partners_riding_changes() {
    let mut flight = PartnerFlight::default();
    for now_ms in (0..10_000).step_by(1_000) {
        flight.step(now_ms, &boarding());
    }
    assert_eq!(flight.step(10_000, &boarding()).call, None);
    flight.step(
        11_000,
        &readings(on_foot(), body(false, FlightMode::Robot, 0)),
    );
    assert_eq!(
        flight.step(12_000, &boarding()).call,
        Some(FlightCall::Board)
    );
}

#[test]
fn a_body_that_rides_stops_the_board_tries() {
    let mut flight = PartnerFlight::default();
    flight.step(0, &boarding());
    let riding = readings(
        partner(FlightKind::FreeFlight, FlightMode::Robot),
        body(true, FlightMode::Robot, FREE_FLIGHT_IDLE_STATE),
    );
    let step = flight.step(100, &riding);
    assert_eq!(step.call, None);
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("the body rides its flight unit (state 0x70)"))
    );
}

#[test]
fn in_a_shooting_stage_the_mode_setter_is_called_when_the_bodys_mode_differs() {
    let mut flight = PartnerFlight::default();
    let step = flight.step(
        0,
        &readings(
            partner(FlightKind::ShootingStage, FlightMode::Fighter),
            body(true, FlightMode::Robot, SHOOTING_IDLE_STATE),
        ),
    );
    assert_eq!(step.call, Some(FlightCall::SetMode(FlightMode::Fighter)));
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("fighter mode setter"))
    );
}

#[test]
fn in_a_shooting_stage_the_mode_setter_brings_a_body_back_into_the_stage_states() {
    let mut flight = PartnerFlight::default();
    let step = flight.step(
        0,
        &readings(
            partner(FlightKind::ShootingStage, FlightMode::Robot),
            body(true, FlightMode::Robot, FREE_FLIGHT_IDLE_STATE),
        ),
    );
    assert_eq!(step.call, Some(FlightCall::SetMode(FlightMode::Robot)));
}

#[test]
fn no_mode_setter_is_called_when_the_body_is_in_the_partners_mode_and_a_stage_state() {
    let mut flight = PartnerFlight::default();
    let step = flight.step(
        0,
        &readings(
            partner(FlightKind::ShootingStage, FlightMode::Robot),
            body(true, FlightMode::Robot, SHOOTING_IDLE_STATE),
        ),
    );
    assert_eq!(step.call, None);
}

#[test]
fn the_mode_setter_is_bounded_and_refused_until_the_partners_mode_changes() {
    let mut flight = PartnerFlight::default();
    let stuck = readings(
        partner(FlightKind::ShootingStage, FlightMode::Fighter),
        body(true, FlightMode::Robot, SHOOTING_IDLE_STATE),
    );
    let mut calls = 0;
    let mut refusals = 0;
    for now_ms in (0..20_000).step_by(250) {
        let step = flight.step(now_ms, &stuck);
        calls += u32::from(step.call.is_some());
        refusals += step
            .logs
            .iter()
            .filter(|line| line.contains("did not take mode fighter"))
            .count();
    }
    assert_eq!(calls, PARTNER_MODE_TRIES);
    assert_eq!(refusals, 1);
    flight.step(
        20_000,
        &readings(
            partner(FlightKind::ShootingStage, FlightMode::Robot),
            body(true, FlightMode::Robot, SHOOTING_IDLE_STATE),
        ),
    );
    assert_eq!(
        flight.step(21_000, &stuck).call,
        Some(FlightCall::SetMode(FlightMode::Fighter))
    );
}

#[test]
fn in_free_flight_no_mode_setter_is_called_and_the_difference_is_logged_once() {
    let mut flight = PartnerFlight::default();
    let free = readings(
        partner(FlightKind::FreeFlight, FlightMode::Fighter),
        body(true, FlightMode::Robot, FREE_FLIGHT_IDLE_STATE),
    );
    let first = flight.step(0, &free);
    assert_eq!(first.call, None);
    assert!(
        first
            .logs
            .iter()
            .any(|line| line.contains("no mode setter is called outside a shooting stage"))
    );
    assert!(flight.step(1_000, &free).logs.is_empty());
}

#[test]
fn while_both_fly_the_unit_is_placed_and_the_partners_stick_is_fed() {
    let mut flight = PartnerFlight::default();
    let step = flight.step(
        0,
        &readings(
            partner(FlightKind::ShootingStage, FlightMode::Robot),
            body(true, FlightMode::Robot, SHOOTING_IDLE_STATE),
        ),
    );
    assert!(step.place_unit);
    assert!(step.body_rides);
    assert_eq!(step.feed_stick, Some([0.25, -0.75]));
}

#[test]
fn a_partners_unit_that_was_not_found_is_not_placed_toward() {
    let mut flight = PartnerFlight::default();
    let unfound = FlightReport {
        unit: None,
        ..partner(FlightKind::FreeFlight, FlightMode::Robot)
    };
    let step = flight.step(
        0,
        &readings(
            unfound,
            body(true, FlightMode::Robot, FREE_FLIGHT_IDLE_STATE),
        ),
    );
    assert!(!step.place_unit);
    assert!(step.feed_stick.is_some());
}

#[test]
fn on_foot_nothing_is_placed_or_fed_and_no_call_is_made() {
    let mut flight = PartnerFlight::default();
    let step = flight.step(
        0,
        &readings(
            on_foot(),
            body(false, FlightMode::Robot, ON_FOOT_IDLE_STATE),
        ),
    );
    assert_eq!(step.call, None);
    assert!(!step.place_unit);
    assert_eq!(step.feed_stick, None);
    assert!(!step.body_rides);
}

#[test]
fn the_unit_is_corrected_by_the_rule_and_limits_of_the_body_position() {
    let mut placer = UnitPlacer::default();
    assert_eq!(
        placer.position_to_write(0, [0.0; 3], [0.05, 0.0, 0.0]),
        None
    );
    let nudged = placer
        .position_to_write(0, [0.0; 3], [0.5, 0.0, 0.0])
        .expect("a nudge");
    assert!(nudged[0] > 0.0 && nudged[0] < 0.5);
    let far = [POSITION_ERROR_TELEPORTED * 10.0, 0.0, 0.0];
    assert_eq!(placer.position_to_write(1_000, [0.0; 3], far), Some(far));
    let again = placer
        .position_to_write(1_100, [0.0; 3], far)
        .expect("a nudge inside the teleport interval");
    assert!(again[0] < far[0]);
}

#[test]
fn the_units_yaw_is_written_only_when_its_error_is_large() {
    assert_eq!(UnitPlacer::yaw_to_write(0.0, 0.2), None);
    assert_eq!(UnitPlacer::yaw_to_write(0.0, 2.5), Some(2.5));
}

#[test]
fn the_body_gets_off_once_the_partners_hero_has_been_out_of_its_unit_for_a_second() {
    let mut flight = PartnerFlight::default();
    let off = readings(
        on_foot(),
        body(true, FlightMode::Robot, FREE_FLIGHT_IDLE_STATE),
    );
    assert_eq!(flight.step(0, &off).call, None);
    assert_eq!(
        flight
            .step(PARTNER_OUT_OF_A_UNIT_BEFORE_THE_BODY_GETS_OFF_MS - 1, &off)
            .call,
        None
    );
    let step = flight.step(PARTNER_OUT_OF_A_UNIT_BEFORE_THE_BODY_GETS_OFF_MS, &off);
    assert_eq!(step.call, Some(FlightCall::GetOff));
    assert!(
        step.logs
            .iter()
            .any(|line| line.contains("endPhaseEventFlightUnit"))
    );
}

#[test]
fn a_get_off_is_bounded_and_then_refused() {
    let mut flight = PartnerFlight::default();
    let off = readings(
        on_foot(),
        body(true, FlightMode::Robot, FREE_FLIGHT_IDLE_STATE),
    );
    let mut calls = 0;
    let mut refusals = 0;
    for now_ms in (0..30_000).step_by(100) {
        let step = flight.step(now_ms, &off);
        calls += u32::from(step.call == Some(FlightCall::GetOff));
        refusals += step
            .logs
            .iter()
            .filter(|line| line.contains("still rides after 5 tries"))
            .count();
    }
    assert_eq!(calls, PARTNER_GET_OFF_TRIES);
    assert_eq!(refusals, 1);
}

#[test]
fn a_riding_body_gets_off_before_it_is_released_only_with_the_stage_on() {
    let riding = body(true, FlightMode::Robot, FREE_FLIGHT_IDLE_STATE);
    assert!(gets_off_before_release(true, Some(riding)));
    assert!(!gets_off_before_release(false, Some(riding)));
    assert!(!gets_off_before_release(
        true,
        Some(body(false, FlightMode::Robot, 0))
    ));
    assert!(!gets_off_before_release(
        true,
        Some(BodyFlight {
            alive: false,
            ..riding
        })
    ));
    assert!(!gets_off_before_release(true, None));
}

#[test]
fn no_flight_state_is_ever_started_on_the_body_by_the_state_list() {
    for state in STATES_STARTED_ON_THE_BODY {
        assert!(!is_a_flight_state(i32::from(state)));
    }
    assert!(is_a_flight_state(0x70) && is_a_flight_state(0x84) && is_a_flight_state(0x9F));
    assert!(!is_a_flight_state(0x6F) && !is_a_flight_state(0xA0));
    assert!(is_a_shooting_stage_state(0x84) && is_a_shooting_stage_state(0x92));
    assert!(!is_a_shooting_stage_state(0x70) && !is_a_shooting_stage_state(0x94));
}

#[test]
fn with_the_stage_off_nothing_is_called_fed_placed_or_logged() {
    let mut flight = PartnerFlight::default();
    let off = FlightReadings {
        applies: false,
        ..boarding()
    };
    for now_ms in (0..5_000).step_by(500) {
        let step = flight.step(now_ms, &off);
        assert_eq!(step, Default::default());
    }
}

#[test]
fn no_call_is_made_on_a_dead_body() {
    let mut flight = PartnerFlight::default();
    let dead = FlightReadings {
        body: Some(BodyFlight {
            alive: false,
            ..body(false, FlightMode::Robot, 0)
        }),
        ..boarding()
    };
    assert_eq!(flight.step(0, &dead).call, None);
}

#[test]
fn a_stale_partner_report_changes_nothing() {
    let mut flight = PartnerFlight::default();
    let stale = FlightReadings {
        partner: None,
        ..boarding()
    };
    let step = flight.step(0, &stale);
    assert_eq!(step.call, None);
    assert_eq!(step.feed_stick, None);
}

#[test]
fn a_riding_mismatch_is_logged_after_a_second_and_when_it_ends() {
    let mut flight = PartnerFlight::default();
    let mismatch = FlightReadings {
        hero_in_a_unit: true,
        ..readings(on_foot(), body(false, FlightMode::Robot, 0))
    };
    assert!(flight.step(0, &mismatch).logs.is_empty());
    let late = flight.step(RIDING_MISMATCH_LOGGED_AFTER_MS, &mismatch);
    assert!(
        late.logs
            .iter()
            .any(|line| line.contains("the hero here is in a flight unit"))
    );
    assert!(flight.step(1_500, &mismatch).logs.is_empty());
    let agreed = flight.step(
        2_000,
        &readings(on_foot(), body(false, FlightMode::Robot, 0)),
    );
    assert!(
        agreed
            .logs
            .iter()
            .any(|line| line.contains("agree again") && line.contains("2000 ms"))
    );
}

#[test]
fn a_short_riding_mismatch_is_not_logged() {
    let mut flight = PartnerFlight::default();
    let mismatch = FlightReadings {
        hero_in_a_unit: true,
        ..readings(on_foot(), body(false, FlightMode::Robot, 0))
    };
    assert!(flight.step(0, &mismatch).logs.is_empty());
    assert!(
        flight
            .step(500, &readings(on_foot(), body(false, FlightMode::Robot, 0)))
            .logs
            .is_empty()
    );
}

#[test]
fn a_shooting_stage_counts_as_in_a_unit_even_with_the_riding_flag_clear() {
    let report = flight_report_of(&FlightFields {
        riding: 0,
        shooting_stage: 1,
        mode_word: 0,
        unit: Some(UnitPose {
            position: [1.0, 2.0, 3.0],
            yaw: 0.25,
        }),
        stick: [0.5, 0.5],
    });
    assert_eq!(report.kind, FlightKind::ShootingStage);
    assert_eq!(report.mode, FlightMode::Robot);
    assert!(!report.riding && report.is_in_a_unit() && report.unit.is_some());
    assert!(report.is_valid());
}

#[test]
fn the_riding_flag_alone_is_free_flight_and_a_mode_word_of_one_is_the_fighter() {
    let report = flight_report_of(&FlightFields {
        riding: 1,
        mode_word: 1,
        ..FlightFields::default()
    });
    assert_eq!(report.kind, FlightKind::FreeFlight);
    assert_eq!(report.mode, FlightMode::Fighter);
    assert!(report.unit.is_none());
    assert_eq!(report.stick, Some([0.0, 0.0]));
    assert!(report.is_valid());
}

#[test]
fn out_of_a_unit_the_report_has_no_mode_and_no_unit() {
    let report = flight_report_of(&FlightFields {
        mode_word: 1,
        unit: Some(UnitPose::default()),
        ..FlightFields::default()
    });
    assert_eq!(report, FlightReport::default());
}

#[test]
fn a_report_is_always_valid_whatever_the_game_held() {
    let report = flight_report_of(&FlightFields {
        riding: 1,
        shooting_stage: 0,
        mode_word: 7,
        unit: Some(UnitPose {
            position: [f32::NAN, 0.0, 0.0],
            yaw: 0.0,
        }),
        stick: [f32::INFINITY, -5_000.0],
    });
    assert!(report.is_valid());
    assert!(report.unit.is_none());
    assert_eq!(report.stick, Some([0.0, -1_000.0]));
    let wrapped = flight_report_of(&FlightFields {
        riding: 1,
        unit: Some(UnitPose {
            position: [0.0; 3],
            yaw: 7.0,
        }),
        ..FlightFields::default()
    });
    assert!(wrapped.is_valid());
    let yaw = wrapped.unit.expect("a unit").yaw;
    assert!((yaw - (7.0 - core::f32::consts::TAU)).abs() < 1e-4);
}

#[test]
fn the_flight_stick_is_picked_as_the_games_flight_movement_picks_it() {
    let choice = StickChoice {
        pad_enable: 1,
        mouse_enable: 1,
        keyboard_enable: 1,
        mouse_length: 1.0,
        keyboard_length: 1.0,
        pad: [1.0, 0.0],
        mouse: [0.0, 1.0],
        keyboard: [-1.0, 0.0],
    };
    assert_eq!(flight_stick_in_use(&choice), [0.0, 1.0]);
    let no_mouse = StickChoice {
        mouse_length: 0.0,
        ..choice
    };
    assert_eq!(flight_stick_in_use(&no_mouse), [-1.0, 0.0]);
    let pad_only = StickChoice {
        keyboard_enable: 0,
        ..no_mouse
    };
    assert_eq!(flight_stick_in_use(&pad_only), [1.0, 0.0]);
    let none = StickChoice {
        pad_enable: 0,
        ..pad_only
    };
    assert_eq!(flight_stick_in_use(&none), [0.0, 0.0]);
}

#[test]
fn bounded_tries_wait_between_tries_and_give_up_once() {
    let mut tries = BoundedTries::default();
    assert_eq!(tries.next(0, 1_000, 2), TryVerdict::Try(1));
    assert_eq!(tries.next(999, 1_000, 2), TryVerdict::Wait);
    assert_eq!(tries.next(1_000, 1_000, 2), TryVerdict::Try(2));
    assert_eq!(tries.next(2_000, 1_000, 2), TryVerdict::GiveUp);
    assert_eq!(tries.next(9_000, 1_000, 2), TryVerdict::Refused);
    tries.reset();
    assert_eq!(tries.tries(), 0);
    assert_eq!(tries.next(9_000, 1_000, 2), TryVerdict::Try(1));
}
