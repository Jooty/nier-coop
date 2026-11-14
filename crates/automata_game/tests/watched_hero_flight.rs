use automata_game::guest_watches::{
    HostFlight, WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS, WATCHED_HERO_FLIGHT_RETRY_MS,
    WATCHED_HERO_FLIGHT_TRIES, WatchedFlightReadings, WatchedHeroFlight, WatchedHeroFlightReading,
    WatchingReading, watching_line,
};
use automata_game::partner_flight::{
    FlightCall, FlightKind, FlightMode, FlightReport, PARTNER_MODE_RETRY_MS, PARTNER_MODE_TRIES,
    UnitPose,
};

const SHOOTING_IDLE_STATE: u16 = 0x84;
const FREE_FLIGHT_IDLE_STATE: u16 = 0x70;
const ON_FOOT_IDLE_STATE: u16 = 0;
const START_MS: u64 = 10_000;

fn report(kind: FlightKind, mode: FlightMode) -> FlightReport {
    FlightReport {
        riding: kind == FlightKind::FreeFlight,
        kind,
        mode: if kind.is_in_a_unit() {
            mode
        } else {
            FlightMode::None
        },
        unit: kind.is_in_a_unit().then_some(UnitPose {
            position: [1.0, 2.0, 3.0],
            yaw: 0.5,
        }),
        stick: kind.is_in_a_unit().then_some([0.0, 0.0]),
    }
}

fn state_of(kind: FlightKind) -> u16 {
    match kind {
        FlightKind::NotInAUnit => ON_FOOT_IDLE_STATE,
        FlightKind::FreeFlight => FREE_FLIGHT_IDLE_STATE,
        FlightKind::ShootingStage => SHOOTING_IDLE_STATE,
    }
}

fn readings(
    host: (FlightKind, FlightMode),
    hero: (FlightKind, FlightMode),
) -> WatchedFlightReadings {
    WatchedFlightReadings {
        applies: true,
        host: Some(HostFlight {
            report: report(host.0, host.1),
            state: state_of(host.0),
        }),
        hero: Some(WatchedHeroFlightReading {
            report: report(hero.0, hero.1),
            state: i32::from(state_of(hero.0)),
            alive: true,
        }),
    }
}

fn host_in_a_stage_hero_on_foot() -> WatchedFlightReadings {
    readings(
        (FlightKind::ShootingStage, FlightMode::Robot),
        (FlightKind::NotInAUnit, FlightMode::None),
    )
}

fn readings_with_host_on_foot() -> WatchedFlightReadings {
    readings(
        (FlightKind::NotInAUnit, FlightMode::None),
        (FlightKind::NotInAUnit, FlightMode::None),
    )
}

fn calls_over(
    rule: &mut WatchedHeroFlight,
    readings: &WatchedFlightReadings,
    from_ms: u64,
    to_ms: u64,
) -> Vec<(u64, FlightCall)> {
    (from_ms..=to_ms)
        .step_by(100)
        .filter_map(|now_ms| rule.step(now_ms, readings).call.map(|call| (now_ms, call)))
        .collect()
}

#[test]
fn the_hero_is_boarded_once_the_host_has_been_in_a_unit_for_the_bound_and_the_hero_not() {
    let mut rule = WatchedHeroFlight::default();
    let readings = host_in_a_stage_hero_on_foot();
    let early = calls_over(
        &mut rule,
        &readings,
        START_MS,
        START_MS + WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS - 100,
    );
    assert!(early.is_empty());
    let step = rule.step(START_MS + WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS, &readings);
    assert_eq!(step.call, Some(FlightCall::Board));
    assert!(step.logs[0].contains("called startPhaseEventFlightUnit on the hero (try 1 of 3)"));
    assert!(step.logs[0].contains("shooting stage in mode robot"));
    assert!(step.logs[0].contains("not in a unit (state 0x0)"));
}

#[test]
fn boarding_is_tried_at_most_three_times_then_refused_until_the_hosts_report_changes() {
    let mut rule = WatchedHeroFlight::default();
    let readings = host_in_a_stage_hero_on_foot();
    let end_ms = START_MS + 10_000;
    let calls = calls_over(&mut rule, &readings, START_MS, end_ms);
    assert_eq!(calls.len(), WATCHED_HERO_FLIGHT_TRIES as usize);
    assert!(
        calls
            .windows(2)
            .all(|pair| pair[1].0 - pair[0].0 >= WATCHED_HERO_FLIGHT_RETRY_MS)
    );
    let mut refused = WatchedHeroFlight::default();
    let lines: Vec<String> = (START_MS..=end_ms)
        .step_by(100)
        .flat_map(|now_ms| refused.step(now_ms, &readings).logs)
        .filter(|line| line.contains("did not follow"))
        .collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("after 3 tries of startPhaseEventFlightUnit"));
    let host_on_foot = readings_with_host_on_foot();
    refused.step(end_ms + 100, &host_on_foot);
    let again = calls_over(&mut refused, &readings, end_ms + 200, end_ms + 5_000);
    assert_eq!(again[0].1, FlightCall::Board);
}

#[test]
fn the_hero_gets_off_in_free_flight_once_the_host_has_been_out_of_a_unit_for_the_bound() {
    let mut rule = WatchedHeroFlight::default();
    let readings = readings(
        (FlightKind::NotInAUnit, FlightMode::None),
        (FlightKind::FreeFlight, FlightMode::Robot),
    );
    let calls = calls_over(
        &mut rule,
        &readings,
        START_MS,
        START_MS + WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS,
    );
    assert_eq!(
        calls,
        vec![(
            START_MS + WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS,
            FlightCall::GetOff
        )]
    );
}

#[test]
fn a_hero_left_in_a_shooting_stage_is_not_called_and_says_why_once() {
    let mut rule = WatchedHeroFlight::default();
    let readings = readings(
        (FlightKind::NotInAUnit, FlightMode::None),
        (FlightKind::ShootingStage, FlightMode::Robot),
    );
    let mut lines = Vec::new();
    for now_ms in (START_MS..=START_MS + 6_000).step_by(100) {
        let step = rule.step(now_ms, &readings);
        assert_eq!(step.call, None);
        lines.extend(step.logs);
    }
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("still in a shooting stage (state 0x84)"));
    assert!(lines[0].contains("nothing is called"));
}

#[test]
fn the_hosts_mode_is_set_on_the_hero_in_a_shooting_stage_after_the_bound_at_most_five_times() {
    let mut rule = WatchedHeroFlight::default();
    let readings = readings(
        (FlightKind::ShootingStage, FlightMode::Fighter),
        (FlightKind::ShootingStage, FlightMode::Robot),
    );
    let calls = calls_over(&mut rule, &readings, START_MS, START_MS + 20_000);
    assert_eq!(calls.len(), PARTNER_MODE_TRIES as usize);
    assert_eq!(
        calls[0],
        (
            START_MS + WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS,
            FlightCall::SetMode(FlightMode::Fighter)
        )
    );
    assert!(
        calls
            .windows(2)
            .all(|pair| pair[1].0 - pair[0].0 >= PARTNER_MODE_RETRY_MS)
    );
    let mut logged = WatchedHeroFlight::default();
    let refusals = (START_MS..=START_MS + 20_000)
        .step_by(100)
        .flat_map(|now_ms| logged.step(now_ms, &readings).logs)
        .filter(|line| line.contains("after 5 tries of the fighter mode setter"))
        .count();
    assert_eq!(refusals, 1);
}

#[test]
fn a_mode_difference_outside_a_shooting_stage_calls_nothing_and_is_logged_once() {
    let mut rule = WatchedHeroFlight::default();
    let readings = readings(
        (FlightKind::FreeFlight, FlightMode::Fighter),
        (FlightKind::FreeFlight, FlightMode::Robot),
    );
    let mut lines = Vec::new();
    for now_ms in (START_MS..=START_MS + 6_000).step_by(100) {
        let step = rule.step(now_ms, &readings);
        assert_eq!(step.call, None);
        lines.extend(step.logs);
    }
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("the mode is left alone"));
}

#[test]
fn the_unit_is_placed_only_while_both_are_in_free_flight() {
    let mut rule = WatchedHeroFlight::default();
    let free = (FlightKind::FreeFlight, FlightMode::Robot);
    let stage = (FlightKind::ShootingStage, FlightMode::Robot);
    assert!(rule.step(START_MS, &readings(free, free)).place_unit);
    assert!(!rule.step(START_MS, &readings(stage, stage)).place_unit);
    assert!(!rule.step(START_MS, &readings(free, stage)).place_unit);
    assert!(!rule.step(START_MS, &readings(stage, free)).place_unit);
}

#[test]
fn a_difference_only_counts_while_the_rule_applies() {
    let mut rule = WatchedHeroFlight::default();
    let readings = host_in_a_stage_hero_on_foot();
    let held = WatchedFlightReadings {
        applies: false,
        ..readings
    };
    rule.step(START_MS, &readings);
    let during_an_event = calls_over(&mut rule, &held, START_MS + 100, START_MS + 5_000);
    assert!(during_an_event.is_empty());
    assert_eq!(rule.step(START_MS + 5_100, &readings).call, None);
    assert_eq!(
        rule.step(
            START_MS + 5_100 + WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS,
            &readings
        )
        .call,
        Some(FlightCall::Board)
    );
}

#[test]
fn nothing_is_called_on_a_hero_read_as_dead_or_absent() {
    let mut rule = WatchedHeroFlight::default();
    let mut dead = host_in_a_stage_hero_on_foot();
    if let Some(hero) = dead.hero.as_mut() {
        hero.alive = false;
    }
    assert!(calls_over(&mut rule, &dead, START_MS, START_MS + 6_000).is_empty());
    let absent = WatchedFlightReadings { hero: None, ..dead };
    assert!(calls_over(&mut rule, &absent, START_MS, START_MS + 6_000).is_empty());
}

#[test]
fn an_empty_host_report_while_the_host_flies_changes_nothing_and_is_logged_once() {
    let mut rule = WatchedHeroFlight::default();
    let mut readings = readings(
        (FlightKind::NotInAUnit, FlightMode::None),
        (FlightKind::FreeFlight, FlightMode::Robot),
    );
    if let Some(host) = readings.host.as_mut() {
        host.state = SHOOTING_IDLE_STATE;
    }
    let mut lines = Vec::new();
    for now_ms in (START_MS..=START_MS + 6_000).step_by(100) {
        let step = rule.step(now_ms, &readings);
        assert_eq!(step.call, None);
        lines.extend(step.logs);
    }
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("its flight report is empty"));
}

#[test]
fn the_hero_is_logged_as_left_on_the_step_after_a_call() {
    let mut rule = WatchedHeroFlight::default();
    let readings = host_in_a_stage_hero_on_foot();
    let call_ms = START_MS + WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS;
    rule.step(START_MS, &readings);
    assert_eq!(rule.step(call_ms, &readings).call, Some(FlightCall::Board));
    let boarded = self::readings(
        (FlightKind::ShootingStage, FlightMode::Robot),
        (FlightKind::FreeFlight, FlightMode::Robot),
    );
    let after = rule.step(call_ms + 100, &boarded);
    assert_eq!(
        after.logs,
        vec![
            "watching: after startPhaseEventFlightUnit the hero is free flight in mode robot (state 0x70)"
                .to_owned()
        ]
    );
}

#[test]
fn an_agreement_starts_the_tries_over() {
    let mut rule = WatchedHeroFlight::default();
    let readings = host_in_a_stage_hero_on_foot();
    let agree = self::readings(
        (FlightKind::ShootingStage, FlightMode::Robot),
        (FlightKind::ShootingStage, FlightMode::Robot),
    );
    let first = calls_over(&mut rule, &readings, START_MS, START_MS + 10_000);
    assert_eq!(first.len(), WATCHED_HERO_FLIGHT_TRIES as usize);
    rule.step(START_MS + 10_100, &agree);
    let again = calls_over(&mut rule, &readings, START_MS + 10_200, START_MS + 20_000);
    assert_eq!(again.len(), WATCHED_HERO_FLIGHT_TRIES as usize);
}

#[test]
fn the_once_a_second_line_names_both_flights_when_the_setting_is_on() {
    let reading = WatchingReading {
        hero_position: None,
        host_position: None,
        hero_state: Some(0x8F),
        host_state: Some(SHOOTING_IDLE_STATE),
        hero_health: None,
        host_health: None,
        hero_rides: true,
        refusal: None,
        flight: Some((
            report(FlightKind::ShootingStage, FlightMode::Robot),
            Some(report(FlightKind::ShootingStage, FlightMode::Fighter)),
        )),
    };
    let line = watching_line(&reading);
    assert!(line.contains(
        "; flight shooting stage in mode robot (hero), shooting stage in mode fighter (host)"
    ));
    assert!(line.ends_with("fed and placed"));
    let off = watching_line(&WatchingReading {
        flight: None,
        ..reading
    });
    assert!(!off.contains("; flight "));
}
