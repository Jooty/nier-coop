use crate::guest_watches::{
    WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS, WATCHED_HERO_FLIGHT_RETRY_MS, WATCHED_HERO_FLIGHT_TRIES,
    hero_flight_after_call_line, hero_flight_mode_left_alone_line, hero_flight_refused_line,
    hero_flight_try_line, hero_stays_in_its_shooting_stage_line, host_flight_report_is_empty_line,
};
use crate::partner_flight::{
    BoundedTries, FlightCall, FlightKind, FlightMode, FlightReport, PARTNER_MODE_RETRY_MS,
    PARTNER_MODE_TRIES, TryVerdict, is_a_flight_state,
};

/// The host's flight as its newest fresh state message tells it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HostFlight {
    pub report: FlightReport,
    pub state: u16,
}

/// This game's watched hero, read this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WatchedHeroFlightReading {
    pub report: FlightReport,
    pub state: i32,
    pub alive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WatchedFlightReadings {
    /// The setting is on, this copy watches, and the hero is fed and placed this tick (both copies
    /// free, one phase, no jump, a fresh host state).
    pub applies: bool,
    pub host: Option<HostFlight>,
    pub hero: Option<WatchedHeroFlightReading>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WatchedFlightStep {
    pub call: Option<FlightCall>,
    /// The hero's flight unit is to be corrected toward the host's unit (both in free flight).
    pub place_unit: bool,
    pub logs: Vec<String>,
}

/// Makes the watched hero's riding and flight mode follow the host's flight report when the
/// hero's own game has not followed for a while; each call bounded, logged as found and as left.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WatchedHeroFlight {
    riding_differs_since_ms: Option<u64>,
    riding_tries: BoundedTries,
    host_in_a_unit: Option<bool>,
    stage_refusal_logged: bool,
    mode_differs_since_ms: Option<u64>,
    mode_tries: BoundedTries,
    host_kind_and_mode: Option<(FlightKind, FlightMode)>,
    modes_left_alone_logged: Option<(FlightMode, FlightMode)>,
    empty_report_logged: bool,
    called: Option<FlightCall>,
}

impl WatchedHeroFlight {
    pub fn step(&mut self, now_ms: u64, readings: &WatchedFlightReadings) -> WatchedFlightStep {
        let mut step = WatchedFlightStep::default();
        let (true, Some(host), Some(hero)) = (readings.applies, readings.host, readings.hero)
        else {
            self.riding_differs_since_ms = None;
            self.mode_differs_since_ms = None;
            return step;
        };
        if let Some(call) = self.called.take() {
            step.logs
                .push(hero_flight_after_call_line(call, &hero.report, hero.state));
        }
        if !hero.alive {
            self.riding_differs_since_ms = None;
            self.mode_differs_since_ms = None;
            return step;
        }
        if !host.report.is_in_a_unit() && is_a_flight_state(i32::from(host.state)) {
            // The host's report is empty while its hero flies: its flight report is switched off.
            if !core::mem::replace(&mut self.empty_report_logged, true) {
                step.logs.push(host_flight_report_is_empty_line(host.state));
            }
            self.riding_differs_since_ms = None;
            self.mode_differs_since_ms = None;
            return step;
        }
        self.empty_report_logged = false;
        self.note_host(&host.report);
        self.follow_riding(now_ms, &host.report, &hero, &mut step);
        if step.call.is_none() {
            self.follow_mode(now_ms, &host.report, &hero, &mut step);
        }
        step.place_unit = host.report.kind == FlightKind::FreeFlight
            && hero.report.kind == FlightKind::FreeFlight
            && host.report.unit.is_some();
        self.called = step.call;
        step
    }

    fn note_host(&mut self, host: &FlightReport) {
        let in_a_unit = host.is_in_a_unit();
        if self.host_in_a_unit != Some(in_a_unit) {
            self.host_in_a_unit = Some(in_a_unit);
            self.riding_tries.reset();
            self.riding_differs_since_ms = None;
            self.stage_refusal_logged = false;
        }
        let kind_and_mode = (host.kind, host.mode);
        if self.host_kind_and_mode != Some(kind_and_mode) {
            self.host_kind_and_mode = Some(kind_and_mode);
            self.mode_tries.reset();
            self.mode_differs_since_ms = None;
            self.modes_left_alone_logged = None;
        }
    }

    fn follow_riding(
        &mut self,
        now_ms: u64,
        host: &FlightReport,
        hero: &WatchedHeroFlightReading,
        step: &mut WatchedFlightStep,
    ) {
        if host.is_in_a_unit() == hero.report.is_in_a_unit() {
            self.riding_differs_since_ms = None;
            self.riding_tries.reset();
            self.stage_refusal_logged = false;
            return;
        }
        let Some(differed_ms) = lasted_long_enough(&mut self.riding_differs_since_ms, now_ms)
        else {
            return;
        };
        let call = if host.is_in_a_unit() {
            FlightCall::Board
        } else if hero.report.kind == FlightKind::ShootingStage {
            // The end native leaves the stage flag set, and the game then starts a stage state.
            if !core::mem::replace(&mut self.stage_refusal_logged, true) {
                step.logs.push(hero_stays_in_its_shooting_stage_line(
                    hero.state,
                    differed_ms,
                ));
            }
            return;
        } else {
            FlightCall::GetOff
        };
        match self.riding_tries.next(
            now_ms,
            WATCHED_HERO_FLIGHT_RETRY_MS,
            WATCHED_HERO_FLIGHT_TRIES,
        ) {
            TryVerdict::Try(number) => {
                step.call = Some(call);
                step.logs.push(hero_flight_try_line(
                    call,
                    host,
                    &hero.report,
                    hero.state,
                    differed_ms,
                    (number, WATCHED_HERO_FLIGHT_TRIES),
                ));
            }
            TryVerdict::GiveUp => step
                .logs
                .push(hero_flight_refused_line(call, WATCHED_HERO_FLIGHT_TRIES)),
            TryVerdict::Wait | TryVerdict::Refused => {}
        }
    }

    fn follow_mode(
        &mut self,
        now_ms: u64,
        host: &FlightReport,
        hero: &WatchedHeroFlightReading,
        step: &mut WatchedFlightStep,
    ) {
        let both_in_a_unit = host.is_in_a_unit() && hero.report.is_in_a_unit();
        if !both_in_a_unit || host.mode == hero.report.mode {
            self.mode_differs_since_ms = None;
            self.mode_tries.reset();
            return;
        }
        let both_in_a_stage =
            host.kind == FlightKind::ShootingStage && hero.report.kind == FlightKind::ShootingStage;
        if !both_in_a_stage {
            // A mode setter starts a shooting stage state; outside a stage it is not called.
            let modes = (host.mode, hero.report.mode);
            if self.modes_left_alone_logged != Some(modes) {
                self.modes_left_alone_logged = Some(modes);
                step.logs
                    .push(hero_flight_mode_left_alone_line(host, &hero.report));
            }
            self.mode_differs_since_ms = None;
            return;
        }
        let Some(differed_ms) = lasted_long_enough(&mut self.mode_differs_since_ms, now_ms) else {
            return;
        };
        let call = FlightCall::SetMode(host.mode);
        match self
            .mode_tries
            .next(now_ms, PARTNER_MODE_RETRY_MS, PARTNER_MODE_TRIES)
        {
            TryVerdict::Try(number) => {
                step.call = Some(call);
                step.logs.push(hero_flight_try_line(
                    call,
                    host,
                    &hero.report,
                    hero.state,
                    differed_ms,
                    (number, PARTNER_MODE_TRIES),
                ));
            }
            TryVerdict::GiveUp => step
                .logs
                .push(hero_flight_refused_line(call, PARTNER_MODE_TRIES)),
            TryVerdict::Wait | TryVerdict::Refused => {}
        }
    }
}

fn lasted_long_enough(differs_since_ms: &mut Option<u64>, now_ms: u64) -> Option<u64> {
    let since_ms = *differs_since_ms.get_or_insert(now_ms);
    let lasted_ms = now_ms.saturating_sub(since_ms);
    (lasted_ms >= WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS).then_some(lasted_ms)
}
