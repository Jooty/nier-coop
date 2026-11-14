use crate::partner_flight::{
    BodyFlight, BoundedTries, FlightCall, FlightCase, FlightKind, FlightMode, FlightReport,
    FlightStep, FlightTakeOver, PARTNER_BOARD_RETRY_MS, PARTNER_BOARD_TRIES,
    PARTNER_GET_OFF_RETRY_MS, PARTNER_GET_OFF_TRIES, PARTNER_MODE_RETRY_MS, PARTNER_MODE_TRIES,
    PARTNER_OUT_OF_A_UNIT_BEFORE_THE_BODY_GETS_OFF_MS, RidingMismatch, TryVerdict,
    board_refused_line, board_try_line, body_got_off_line, body_rides_line,
    free_flight_mode_left_alone_line, get_off_refused_line, get_off_try_line,
    is_a_shooting_stage_state, mode_refused_line, mode_try_line,
};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FlightReadings {
    /// The setting is on, the session is the campaign and the body is driven this tick.
    pub applies: bool,
    /// The other player's newest fresh report; none while its state is stale.
    pub partner: Option<FlightReport>,
    /// None when no body is held or it could not be read.
    pub body: Option<BodyFlight>,
    pub hero_in_a_unit: bool,
}

/// Keeps the partner body in a flight unit while the other player's hero is in one. The default
/// is stage C5's rule; `with_take_over(true)` adds the switch of a body the game flies itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartnerFlight {
    take_over: bool,
    flight_take_over: FlightTakeOver,
    board: BoundedTries,
    mode: BoundedTries,
    get_off: BoundedTries,
    partner_in_a_unit: Option<bool>,
    partner_kind_and_mode: Option<(FlightKind, FlightMode)>,
    partner_out_since_ms: Option<u64>,
    body_rode: Option<bool>,
    free_flight_modes_logged: Option<(FlightMode, FlightMode)>,
    mismatch: RidingMismatch,
}

impl PartnerFlight {
    pub fn with_take_over(take_over: bool) -> Self {
        Self {
            take_over,
            ..Self::default()
        }
    }

    pub fn step(&mut self, now_ms: u64, readings: &FlightReadings) -> FlightStep {
        let mut step = FlightStep::default();
        if !readings.applies {
            return step;
        }
        let Some(body) = readings.body else {
            *self = Self::with_take_over(self.take_over);
            return step;
        };
        step.body_rides = body.riding;
        if let Some(line) = self
            .mismatch
            .observe(now_ms, readings.hero_in_a_unit, body.riding)
        {
            step.logs.push(line);
        }
        self.note_body(body, &mut step);
        if !body.alive {
            return step;
        }
        let Some(partner) = readings.partner else {
            return step;
        };
        let flight_started = self.note_partner(now_ms, &partner);
        let taken_over = self.take_over && body.is_in_a_unit();
        if self.take_over && partner.is_in_a_unit() {
            if flight_started {
                self.flight_take_over
                    .judge(now_ms, partner.kind, &body, &mut step);
            } else if taken_over {
                self.flight_take_over.watch(now_ms, &body, &mut step);
            }
        }
        match (partner.is_in_a_unit(), body.riding) {
            (true, false) if !taken_over => self.board(now_ms, partner.kind, &mut step),
            (false, true) => self.get_off(now_ms, &mut step),
            (true, true) if !self.flight_take_over.left_to_the_game() => {
                self.fly(now_ms, &partner, body, &mut step);
            }
            _ => {}
        }
        step
    }

    /// The take-over's judgement of the partner's current flight; none with the take-over off.
    pub fn flight_case(&self) -> Option<FlightCase> {
        self.flight_take_over.case()
    }

    fn note_body(&mut self, body: BodyFlight, step: &mut FlightStep) {
        if self.body_rode == Some(body.riding) {
            return;
        }
        if self.body_rode.is_some() {
            step.logs.push(if body.riding {
                body_rides_line(body.state)
            } else {
                body_got_off_line(body.state)
            });
        }
        self.body_rode = Some(body.riding);
        if body.riding {
            self.board.reset();
        } else {
            self.get_off.reset();
            self.mode.reset();
        }
    }

    fn note_partner(&mut self, now_ms: u64, partner: &FlightReport) -> bool {
        let in_a_unit = partner.is_in_a_unit();
        let changed = self.partner_in_a_unit != Some(in_a_unit);
        if changed {
            self.partner_in_a_unit = Some(in_a_unit);
            self.board.reset();
            self.get_off.reset();
        }
        if in_a_unit {
            self.partner_out_since_ms = None;
        } else {
            self.partner_out_since_ms.get_or_insert(now_ms);
        }
        let kind_and_mode = (partner.kind, partner.mode);
        if self.partner_kind_and_mode != Some(kind_and_mode) {
            self.partner_kind_and_mode = Some(kind_and_mode);
            self.mode.reset();
            self.free_flight_modes_logged = None;
        }
        changed && in_a_unit
    }

    fn board(&mut self, now_ms: u64, kind: FlightKind, step: &mut FlightStep) {
        match self
            .board
            .next(now_ms, PARTNER_BOARD_RETRY_MS, PARTNER_BOARD_TRIES)
        {
            TryVerdict::Try(number) => {
                step.call = Some(FlightCall::Board);
                step.logs
                    .push(board_try_line(kind, number, PARTNER_BOARD_TRIES));
            }
            TryVerdict::GiveUp => step.logs.push(board_refused_line(PARTNER_BOARD_TRIES)),
            TryVerdict::Wait | TryVerdict::Refused => {}
        }
    }

    fn get_off(&mut self, now_ms: u64, step: &mut FlightStep) {
        let out_long_enough = self.partner_out_since_ms.is_some_and(|since| {
            now_ms.saturating_sub(since) >= PARTNER_OUT_OF_A_UNIT_BEFORE_THE_BODY_GETS_OFF_MS
        });
        if !out_long_enough {
            return;
        }
        match self
            .get_off
            .next(now_ms, PARTNER_GET_OFF_RETRY_MS, PARTNER_GET_OFF_TRIES)
        {
            TryVerdict::Try(number) => {
                step.call = Some(FlightCall::GetOff);
                step.logs
                    .push(get_off_try_line(number, PARTNER_GET_OFF_TRIES));
            }
            TryVerdict::GiveUp => step.logs.push(get_off_refused_line(PARTNER_GET_OFF_TRIES)),
            TryVerdict::Wait | TryVerdict::Refused => {}
        }
    }

    fn fly(
        &mut self,
        now_ms: u64,
        partner: &FlightReport,
        body: BodyFlight,
        step: &mut FlightStep,
    ) {
        step.place_unit = partner.unit.is_some();
        step.feed_stick = partner.stick;
        match partner.kind {
            FlightKind::ShootingStage => self.keep_the_stage_mode(now_ms, partner.mode, body, step),
            FlightKind::FreeFlight => {
                let modes = (partner.mode, body.mode);
                if partner.mode != body.mode && self.free_flight_modes_logged != Some(modes) {
                    self.free_flight_modes_logged = Some(modes);
                    step.logs
                        .push(free_flight_mode_left_alone_line(partner.mode, body.mode));
                }
            }
            FlightKind::NotInAUnit => {}
        }
    }

    fn keep_the_stage_mode(
        &mut self,
        now_ms: u64,
        mode: FlightMode,
        body: BodyFlight,
        step: &mut FlightStep,
    ) {
        if body.mode == mode && is_a_shooting_stage_state(body.state) {
            self.mode.reset();
            return;
        }
        match self
            .mode
            .next(now_ms, PARTNER_MODE_RETRY_MS, PARTNER_MODE_TRIES)
        {
            TryVerdict::Try(number) => {
                step.call = Some(FlightCall::SetMode(mode));
                step.logs.push(mode_try_line(
                    mode,
                    body.mode,
                    body.state,
                    number,
                    PARTNER_MODE_TRIES,
                ));
            }
            TryVerdict::GiveUp => step.logs.push(mode_refused_line(mode, PARTNER_MODE_TRIES)),
            TryVerdict::Wait | TryVerdict::Refused => {}
        }
    }
}

/// The body is to be taken off its unit before the mod releases or removes it.
pub fn gets_off_before_release(applies: bool, body: Option<BodyFlight>) -> bool {
    applies && body.is_some_and(|body| body.riding && body.alive)
}
