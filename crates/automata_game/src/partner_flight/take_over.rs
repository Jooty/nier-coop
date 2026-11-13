use crate::partner_flight::{
    BodyFlight, BoundedTries, COMPUTER_FLIGHT_STATE_AFTER_A_SWITCH_LOGGED_AFTER_MS,
    COMPUTER_FLIGHT_SWITCH_RETRY_MS, COMPUTER_FLIGHT_SWITCH_TRIES_PER_FLIGHT, FlightCase,
    FlightKind, FlightSteering, FlightStep, TryVerdict, flight_start_line,
    is_a_computer_flight_state, still_computer_flight_after_a_switch_line, switch_refused_line,
    switched_again_line,
};

/// Switches a partner body the game flies by computer to player flight, for one flight of the
/// partner at a time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlightTakeOver {
    case: Option<FlightCase>,
    later_switches: BoundedTries,
    left_to_the_game: bool,
    switched_at_ms: Option<u64>,
}

impl FlightTakeOver {
    /// The partner's flight started: judges once who put the body in its unit, and switches a
    /// body the game already flies by computer.
    pub fn judge(
        &mut self,
        now_ms: u64,
        kind: FlightKind,
        body: &BodyFlight,
        step: &mut FlightStep,
    ) {
        *self = Self::default();
        let case = if body.is_in_a_unit() {
            FlightCase::TakenOverFromTheGame
        } else {
            FlightCase::MadeByTheMod
        };
        self.case = Some(case);
        let mut left = body.steering;
        if case == FlightCase::TakenOverFromTheGame && body.steering == FlightSteering::Computer {
            step.steer_as_player = true;
            self.switched_at_ms = Some(now_ms);
            left = FlightSteering::Player;
        }
        step.logs.push(flight_start_line(kind, case, body, left));
    }

    /// While the partner flies and the body is in a unit: switches the body again when the game
    /// has turned it back to computer flight, a bounded number of times per flight.
    pub fn watch(&mut self, now_ms: u64, body: &BodyFlight, step: &mut FlightStep) {
        if self.left_to_the_game {
            return;
        }
        if body.steering != FlightSteering::Computer {
            self.note_state_after_the_switch(now_ms, body, step);
            return;
        }
        match self.later_switches.next(
            now_ms,
            COMPUTER_FLIGHT_SWITCH_RETRY_MS,
            COMPUTER_FLIGHT_SWITCH_TRIES_PER_FLIGHT,
        ) {
            TryVerdict::Try(number) => {
                step.steer_as_player = true;
                self.switched_at_ms = Some(now_ms);
                step.logs.push(switched_again_line(
                    body.state,
                    number,
                    COMPUTER_FLIGHT_SWITCH_TRIES_PER_FLIGHT,
                ));
            }
            TryVerdict::GiveUp => {
                self.left_to_the_game = true;
                self.switched_at_ms = None;
                step.logs
                    .push(switch_refused_line(COMPUTER_FLIGHT_SWITCH_TRIES_PER_FLIGHT));
            }
            TryVerdict::Wait | TryVerdict::Refused => {}
        }
    }

    pub fn case(&self) -> Option<FlightCase> {
        self.case
    }

    /// The switch was refused in this flight: the mod neither steers nor places the body.
    pub fn left_to_the_game(&self) -> bool {
        self.left_to_the_game
    }

    fn note_state_after_the_switch(
        &mut self,
        now_ms: u64,
        body: &BodyFlight,
        step: &mut FlightStep,
    ) {
        let Some(switched_at_ms) = self.switched_at_ms else {
            return;
        };
        let after_ms = now_ms.saturating_sub(switched_at_ms);
        if after_ms < COMPUTER_FLIGHT_STATE_AFTER_A_SWITCH_LOGGED_AFTER_MS {
            return;
        }
        self.switched_at_ms = None;
        if is_a_computer_flight_state(body.state) {
            step.logs.push(still_computer_flight_after_a_switch_line(
                body.state, after_ms,
            ));
        }
    }
}
