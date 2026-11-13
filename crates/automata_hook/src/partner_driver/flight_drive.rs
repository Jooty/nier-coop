use automata_game::partner_flight::{
    FlightCall, FlightReadings, FlightReport, PartnerFlight, UnitPlacer, UnitPose,
    flight_report_of, get_off_before_release_line, gets_off_before_release,
    partner_flight_take_over_setting_line, partner_flight_unit_setting_line,
    steering_write_failed_line, unit_placed_line,
};
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::{clear_flight_stick_feed, set_flight_stick_feed};
use crate::net::set_local_flight_report;

/// The partner body's flight unit on this game, run from the partner driver's tick.
pub(crate) struct FlightDrive {
    on: bool,
    take_over: bool,
    campaign: bool,
    hero_in_a_unit: bool,
    machine: PartnerFlight,
    placer: UnitPlacer,
    unit_placed_logged: bool,
}

impl FlightDrive {
    pub(crate) fn new(on: bool, take_over: bool) -> Self {
        log_line(&partner_flight_unit_setting_line(on));
        if on {
            log_line(&partner_flight_take_over_setting_line(take_over));
        }
        Self {
            on,
            take_over,
            campaign: false,
            hero_in_a_unit: false,
            machine: PartnerFlight::with_take_over(take_over),
            placer: UnitPlacer::default(),
            unit_placed_logged: false,
        }
    }

    pub(crate) fn applies(&self) -> bool {
        self.on && self.campaign
    }

    /// Reads the hero's flight unit and hands it to every state message from now on; an empty
    /// report while the stage does not apply.
    pub(crate) fn publish_hero(
        &mut self,
        calls: &GameCalls,
        hero_body: Option<usize>,
        campaign: bool,
    ) {
        self.campaign = campaign;
        let report = hero_body
            .filter(|_| self.applies())
            .and_then(|body| calls.read_flight_fields(body))
            .map(|fields| flight_report_of(&fields))
            .unwrap_or_default();
        self.hero_in_a_unit = report.is_in_a_unit();
        set_local_flight_report(report);
    }

    /// Starts over on a newly held body.
    pub(crate) fn restart(&mut self) {
        self.machine = PartnerFlight::with_take_over(self.take_over);
        self.placer.restart();
    }

    /// One step on the driven body; true when the body rides a flight unit after it.
    #[expect(
        unsafe_code,
        reason = "calls the game's flight unit start, end and mode setters"
    )]
    pub(crate) fn step(
        &mut self,
        calls: &GameCalls,
        body: usize,
        now_ms: u64,
        partner: Option<FlightReport>,
        stick_is_fed: bool,
    ) -> bool {
        let readings = FlightReadings {
            applies: self.applies(),
            partner,
            body: calls.read_body_flight(body),
            hero_in_a_unit: self.hero_in_a_unit,
        };
        let step = self.machine.step(now_ms, &readings);
        for line in &step.logs {
            log_line(line);
        }
        // The steering word goes first: a mode setter's state 0x84 must not fall back to 0x94.
        if step.steer_as_player && !calls.write_player_flight_steering(body) {
            log_line(&steering_write_failed_line());
        }
        match step.call {
            // SAFETY: the body was resolved from its live handle this tick on the game thread,
            // and the rule makes calls only on a body it read as alive.
            Some(FlightCall::Board) => unsafe { calls.start_phase_event_flight_unit(body) },
            // SAFETY: as above; the rule asks for a mode only while the body rides.
            Some(FlightCall::SetMode(mode)) => unsafe { calls.set_flight_mode(body, mode) },
            // SAFETY: as above.
            Some(FlightCall::GetOff) => unsafe { calls.end_phase_event_flight_unit(body) },
            None => {}
        }
        match step.feed_stick.filter(|_| stick_is_fed) {
            Some(stick) if calls.open_flight_pad_gate(body) => set_flight_stick_feed(stick),
            _ => clear_flight_stick_feed(),
        }
        if step.place_unit
            && let Some(remote) = partner.and_then(|partner| partner.unit)
        {
            self.place_unit(calls, body, now_ms, remote);
        }
        step.body_rides
    }

    /// Takes a riding body off its unit before the mod releases or removes it.
    #[expect(unsafe_code, reason = "calls the game's flight unit end")]
    pub(crate) fn before_release(&mut self, calls: &GameCalls, body: Option<usize>) {
        clear_flight_stick_feed();
        let Some(body) = body else {
            return;
        };
        if gets_off_before_release(self.applies(), calls.read_body_flight(body)) {
            log_line(&get_off_before_release_line());
            // SAFETY: the body was resolved from its live handle this tick on the game thread
            // and read as alive and riding.
            unsafe { calls.end_phase_event_flight_unit(body) };
        }
        self.restart();
    }

    fn place_unit(&mut self, calls: &GameCalls, body: usize, now_ms: u64, remote: UnitPose) {
        let unit = calls.flight_unit_of(body);
        let Some(local) = calls.read_unit_pose(unit) else {
            return;
        };
        if let Some(position) =
            self.placer
                .position_to_write(now_ms, local.position, remote.position)
            && calls.write_unit_position(unit, position)
            && !self.unit_placed_logged
        {
            self.unit_placed_logged = true;
            log_line(&unit_placed_line(position == remote.position));
        }
        if let Some(yaw) = UnitPlacer::yaw_to_write(local.yaw, remote.yaw) {
            calls.write_unit_yaw(unit, yaw);
        }
    }
}
