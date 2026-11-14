use automata_game::partner_flight::{
    BodyFlight, COMPUTER_FLIGHT_STATE_AFTER_A_SWITCH_LOGGED_AFTER_MS,
    COMPUTER_FLIGHT_SWITCH_RETRY_MS, COMPUTER_FLIGHT_SWITCH_TRIES_PER_FLIGHT, FlightCall,
    FlightCase, FlightKind, FlightMode, FlightReadings, FlightReport, FlightSteering, FlightStep,
    PARTNER_OUT_OF_A_UNIT_BEFORE_THE_BODY_GETS_OFF_MS, PartnerFlight, UnitPose,
    is_a_computer_flight_state,
};

const COMPUTER_FLIGHT_IDLE_STATE: i32 = 0x94;
const FREE_FLIGHT_IDLE_STATE: i32 = 0x70;
const SHOOTING_IDLE_STATE: i32 = 0x84;
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

fn on_foot_body() -> BodyFlight {
    BodyFlight {
        riding: false,
        mode: FlightMode::Robot,
        state: ON_FOOT_IDLE_STATE,
        alive: true,
        shooting_stage: false,
        unit_found: false,
        steering: FlightSteering::Player,
    }
}

fn boarded_by_the_game(mode: FlightMode) -> BodyFlight {
    BodyFlight {
        riding: true,
        mode,
        state: COMPUTER_FLIGHT_IDLE_STATE,
        unit_found: true,
        steering: FlightSteering::Computer,
        ..on_foot_body()
    }
}

fn switched(body: BodyFlight, state: i32) -> BodyFlight {
    BodyFlight {
        steering: FlightSteering::Player,
        state,
        ..body
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

fn free_flight() -> FlightReport {
    partner(FlightKind::FreeFlight, FlightMode::Fighter)
}

fn has_line(step: &FlightStep, text: &str) -> bool {
    step.logs.iter().any(|line| line.contains(text))
}

fn started_on_a_body_the_game_flies(flight: &mut PartnerFlight) -> FlightStep {
    flight.step(
        0,
        &readings(free_flight(), boarded_by_the_game(FlightMode::Fighter)),
    )
}

#[test]
fn a_body_out_of_a_unit_at_the_flight_start_is_made_by_the_mod_and_boarded_as_c5_does() {
    let mut flight = PartnerFlight::with_take_over(true);
    let step = flight.step(
        0,
        &readings(
            partner(FlightKind::ShootingStage, FlightMode::Robot),
            on_foot_body(),
        ),
    );
    assert_eq!(step.call, Some(FlightCall::Board));
    assert!(!step.steer_as_player);
    assert_eq!(flight.flight_case(), Some(FlightCase::MadeByTheMod));
    assert!(has_line(
        &step,
        "the body is not in a unit (riding false, shooting stage false, unit found false): made by the mod, it is boarded as stage C5 does; flight steering field 0x16488 found 1 (player) and left 1 (player); body state 0x0"
    ));
}

#[test]
fn a_body_the_game_boarded_is_not_boarded_again_but_switched_to_player_flight() {
    let mut flight = PartnerFlight::with_take_over(true);
    let step = started_on_a_body_the_game_flies(&mut flight);
    assert_ne!(step.call, Some(FlightCall::Board));
    assert!(step.steer_as_player);
    assert_eq!(flight.flight_case(), Some(FlightCase::TakenOverFromTheGame));
    assert!(has_line(
        &step,
        "the body is already in a unit (riding true, shooting stage false, unit found true): taken over from the game, no board call; flight steering field 0x16488 found 2 (computer) and left 1 (player); body state 0x94"
    ));
}

#[test]
fn a_shooting_stage_flag_or_a_unit_that_resolves_is_in_a_unit_without_the_riding_flag() {
    for body in [
        BodyFlight {
            shooting_stage: true,
            ..on_foot_body()
        },
        BodyFlight {
            unit_found: true,
            ..on_foot_body()
        },
    ] {
        let mut flight = PartnerFlight::with_take_over(true);
        for now_ms in (0..10_000).step_by(500) {
            let step = flight.step(now_ms, &readings(free_flight(), body));
            assert_ne!(step.call, Some(FlightCall::Board));
        }
        assert_eq!(flight.flight_case(), Some(FlightCase::TakenOverFromTheGame));
    }
}

#[test]
fn a_taken_over_body_already_in_player_flight_is_left_as_found() {
    let mut flight = PartnerFlight::with_take_over(true);
    let step = flight.step(
        0,
        &readings(
            free_flight(),
            switched(
                boarded_by_the_game(FlightMode::Fighter),
                FREE_FLIGHT_IDLE_STATE,
            ),
        ),
    );
    assert!(!step.steer_as_player);
    assert!(has_line(&step, "found 1 (player) and left 1 (player)"));
}

#[test]
fn in_a_shooting_stage_the_switch_comes_with_c5s_mode_setter_out_of_the_computer_state() {
    let mut flight = PartnerFlight::with_take_over(true);
    let step = flight.step(
        0,
        &readings(
            partner(FlightKind::ShootingStage, FlightMode::Fighter),
            boarded_by_the_game(FlightMode::Fighter),
        ),
    );
    assert!(step.steer_as_player);
    assert_eq!(step.call, Some(FlightCall::SetMode(FlightMode::Fighter)));
}

#[test]
fn mode_unit_placing_and_stick_stay_c5s_on_a_taken_over_body() {
    let mut flight = PartnerFlight::with_take_over(true);
    let step = started_on_a_body_the_game_flies(&mut flight);
    assert!(step.place_unit);
    assert_eq!(step.feed_stick, Some([0.25, -0.75]));
    assert!(step.body_rides);
}

#[test]
fn the_case_is_judged_once_per_flight_and_again_at_the_next_one() {
    let mut flight = PartnerFlight::with_take_over(true);
    let first = started_on_a_body_the_game_flies(&mut flight);
    assert!(has_line(&first, "started a flight"));
    let riding = switched(
        boarded_by_the_game(FlightMode::Fighter),
        FREE_FLIGHT_IDLE_STATE,
    );
    for now_ms in (100..3_000).step_by(100) {
        let step = flight.step(now_ms, &readings(free_flight(), riding));
        assert!(!has_line(&step, "started a flight"));
    }
    flight.step(3_000, &readings(FlightReport::default(), riding));
    let off = BodyFlight {
        riding: false,
        unit_found: false,
        ..riding
    };
    flight.step(4_000, &readings(FlightReport::default(), off));
    let next = flight.step(5_000, &readings(free_flight(), off));
    assert!(has_line(&next, "made by the mod"));
    assert_eq!(flight.flight_case(), Some(FlightCase::MadeByTheMod));
}

#[test]
fn when_the_game_turns_the_body_back_to_computer_flight_it_is_switched_again() {
    let mut flight = PartnerFlight::with_take_over(true);
    started_on_a_body_the_game_flies(&mut flight);
    let back_to_computer = boarded_by_the_game(FlightMode::Fighter);
    let step = flight.step(2_000, &readings(free_flight(), back_to_computer));
    assert!(step.steer_as_player);
    assert!(has_line(
        &step,
        "the game turned the body back to computer flight (field 0x16488 2 (computer), state 0x94) while the other player flies; set it to 1 (player) again (switch 1 of 5)"
    ));
}

#[test]
fn a_body_made_by_the_mod_is_switched_when_the_game_boards_it_later_in_the_flight() {
    let mut flight = PartnerFlight::with_take_over(true);
    flight.step(0, &readings(free_flight(), on_foot_body()));
    let step = flight.step(
        500,
        &readings(free_flight(), boarded_by_the_game(FlightMode::Fighter)),
    );
    assert!(step.steer_as_player);
    assert!(has_line(&step, "switch 1 of 5"));
    assert_eq!(flight.flight_case(), Some(FlightCase::MadeByTheMod));
}

#[test]
fn later_switches_are_bounded_per_flight_then_the_body_is_left_to_the_game() {
    let mut flight = PartnerFlight::with_take_over(true);
    started_on_a_body_the_game_flies(&mut flight);
    let back_to_computer = boarded_by_the_game(FlightMode::Fighter);
    let mut switches = 0;
    let mut refusals = 0;
    let mut after_refusal = Vec::new();
    for now_ms in (100..20_000).step_by(100) {
        let step = flight.step(now_ms, &readings(free_flight(), back_to_computer));
        if step.steer_as_player {
            switches += 1;
            assert_eq!(now_ms % COMPUTER_FLIGHT_SWITCH_RETRY_MS, 100);
        }
        if has_line(&step, "the body is left to the game") {
            refusals += 1;
        } else if refusals > 0 {
            after_refusal.push(step);
        }
    }
    assert_eq!(switches, COMPUTER_FLIGHT_SWITCH_TRIES_PER_FLIGHT);
    assert_eq!(refusals, 1);
    for step in after_refusal {
        assert!(!step.steer_as_player && !step.place_unit);
        assert_eq!(step.call, None);
        assert_eq!(step.feed_stick, None);
    }
}

#[test]
fn a_body_left_to_the_game_is_taken_over_again_at_the_next_flight() {
    let mut flight = PartnerFlight::with_take_over(true);
    started_on_a_body_the_game_flies(&mut flight);
    let back_to_computer = boarded_by_the_game(FlightMode::Fighter);
    for now_ms in (1_000..10_000).step_by(1_000) {
        flight.step(now_ms, &readings(free_flight(), back_to_computer));
    }
    flight.step(10_000, &readings(FlightReport::default(), back_to_computer));
    let step = flight.step(11_000, &readings(free_flight(), back_to_computer));
    assert!(step.steer_as_player);
    assert!(step.place_unit);
    assert!(has_line(&step, "taken over from the game"));
}

#[test]
fn a_body_still_in_a_computer_state_a_second_after_the_switch_is_logged_once() {
    let mut flight = PartnerFlight::with_take_over(true);
    started_on_a_body_the_game_flies(&mut flight);
    let still_computer = switched(
        boarded_by_the_game(FlightMode::Fighter),
        COMPUTER_FLIGHT_IDLE_STATE,
    );
    let mut lines = 0;
    for now_ms in (100..5_000).step_by(100) {
        let step = flight.step(now_ms, &readings(free_flight(), still_computer));
        if has_line(&step, "still runs computer flight state 0x94") {
            lines += 1;
            assert!(now_ms >= COMPUTER_FLIGHT_STATE_AFTER_A_SWITCH_LOGGED_AFTER_MS);
        }
    }
    assert_eq!(lines, 1);
}

#[test]
fn a_body_that_left_the_computer_state_after_the_switch_is_not_logged() {
    let mut flight = PartnerFlight::with_take_over(true);
    started_on_a_body_the_game_flies(&mut flight);
    let flying = switched(
        boarded_by_the_game(FlightMode::Fighter),
        FREE_FLIGHT_IDLE_STATE,
    );
    for now_ms in (100..5_000).step_by(100) {
        let step = flight.step(now_ms, &readings(free_flight(), flying));
        assert!(!has_line(&step, "still runs computer flight"));
    }
}

#[test]
fn the_get_off_stays_c5s_with_the_take_over_on() {
    let mut flight = PartnerFlight::with_take_over(true);
    started_on_a_body_the_game_flies(&mut flight);
    let riding = switched(
        boarded_by_the_game(FlightMode::Fighter),
        FREE_FLIGHT_IDLE_STATE,
    );
    flight.step(1_000, &readings(FlightReport::default(), riding));
    let step = flight.step(
        1_000 + PARTNER_OUT_OF_A_UNIT_BEFORE_THE_BODY_GETS_OFF_MS,
        &readings(FlightReport::default(), riding),
    );
    assert_eq!(step.call, Some(FlightCall::GetOff));
    assert!(!step.steer_as_player);
}

#[test]
fn no_switch_on_a_dead_body_a_stale_report_or_outside_the_campaign() {
    let dead = BodyFlight {
        alive: false,
        ..boarded_by_the_game(FlightMode::Fighter)
    };
    let mut flight = PartnerFlight::with_take_over(true);
    assert_eq!(
        flight.step(0, &readings(free_flight(), dead)),
        FlightStep {
            body_rides: true,
            ..FlightStep::default()
        }
    );
    let mut flight = PartnerFlight::with_take_over(true);
    let stale = FlightReadings {
        partner: None,
        ..readings(free_flight(), boarded_by_the_game(FlightMode::Fighter))
    };
    assert!(!flight.step(0, &stale).steer_as_player);
    let mut flight = PartnerFlight::with_take_over(true);
    let off = FlightReadings {
        applies: false,
        ..readings(free_flight(), boarded_by_the_game(FlightMode::Fighter))
    };
    assert_eq!(flight.step(0, &off), FlightStep::default());
}

#[test]
fn with_the_take_over_off_the_flight_is_c5s_exactly() {
    let taken_over_states = [
        boarded_by_the_game(FlightMode::Fighter),
        BodyFlight {
            unit_found: true,
            ..on_foot_body()
        },
    ];
    for body in taken_over_states {
        for partner in [
            free_flight(),
            partner(FlightKind::ShootingStage, FlightMode::Fighter),
        ] {
            let mut c5 = PartnerFlight::default();
            let mut off = PartnerFlight::with_take_over(false);
            for now_ms in (0..10_000).step_by(250) {
                let step = off.step(now_ms, &readings(partner, body));
                assert!(!step.steer_as_player);
                assert!(!has_line(&step, "started a flight"));
                assert_eq!(step, c5.step(now_ms, &readings(partner, body)));
            }
            assert_eq!(off.flight_case(), None);
        }
    }
    let mut off = PartnerFlight::with_take_over(false);
    let step = off.step(
        0,
        &readings(
            free_flight(),
            BodyFlight {
                unit_found: true,
                ..on_foot_body()
            },
        ),
    );
    assert_eq!(step.call, Some(FlightCall::Board));
}

#[test]
fn the_take_over_is_kept_when_the_body_is_lost_and_found_again() {
    let mut flight = PartnerFlight::with_take_over(true);
    let no_body = FlightReadings {
        body: None,
        ..readings(free_flight(), on_foot_body())
    };
    flight.step(0, &no_body);
    assert!(started_on_a_body_the_game_flies(&mut flight).steer_as_player);
}

#[test]
fn the_steering_word_and_the_computer_flight_states_read_as_the_game_has_them() {
    assert_eq!(FlightSteering::of_word(1), FlightSteering::Player);
    assert_eq!(FlightSteering::of_word(2), FlightSteering::Computer);
    assert_eq!(FlightSteering::of_word(0), FlightSteering::Other(0));
    assert_eq!(FlightSteering::Player.word(), 1);
    assert_eq!(FlightSteering::Computer.describe(), "2 (computer)");
    assert_eq!(FlightSteering::Other(7).describe(), "7 (neither)");
    assert!(is_a_computer_flight_state(0x94) && is_a_computer_flight_state(0x9F));
    assert!(!is_a_computer_flight_state(0x93) && !is_a_computer_flight_state(0xA0));
    assert!(!is_a_computer_flight_state(SHOOTING_IDLE_STATE));
}
