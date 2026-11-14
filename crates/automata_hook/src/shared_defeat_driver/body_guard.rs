use automata_game::defeat::LOWEST_HEALTH_KEPT_ON_THE_PARTNER_BODY;
use automata_game::minute_line_budget::MinuteLineBudget;
use automata_game::partner::BODY_HEALTH_OFFSET;
use automata_game::shared_defeat::{
    PARTNER_BODY_GUARD_LINES_LOGGED_PER_MINUTE, PartnerBodyGuardAct, partner_body_guard_act,
    partner_body_guard_line,
};

use crate::budgeted_log::log_owned_within;
use crate::game_calls::GameCalls;
use crate::partner_driver::held_partner_body;

/// Keeps the held partner body out of the two paths that ask for this copy's defeat flow
/// without testing whose body it is; game thread only.
#[derive(Debug)]
pub(super) struct BodyGuard {
    lines: MinuteLineBudget,
}

impl BodyGuard {
    pub(super) const fn new() -> Self {
        Self {
            lines: MinuteLineBudget::new(PARTNER_BODY_GUARD_LINES_LOGGED_PER_MINUTE),
        }
    }

    pub(super) fn keep_the_partner_body_out_of_this_copys_flow(&mut self, calls: &GameCalls) {
        let Some(body) = held_partner_body() else {
            return;
        };
        let Some(reading) = calls.read_partner_body_for_the_guard(body) else {
            return;
        };
        let act = partner_body_guard_act(reading);
        let written = match act {
            PartnerBodyGuardAct::Nothing => return,
            PartnerBodyGuardAct::DisarmTheFlightDefeatTimer => {
                calls.disarm_the_flight_defeat_timer(body)
            }
            PartnerBodyGuardAct::KeepTheHealthAtOne => calls
                .write_i32_at(
                    calls.body_field(body, BODY_HEALTH_OFFSET),
                    LOWEST_HEALTH_KEPT_ON_THE_PARTNER_BODY,
                )
                .is_ok(),
        };
        if let Some(line) = partner_body_guard_line(act, reading, written) {
            log_owned_within(&mut self.lines, "shared defeat: partner body guard", line);
        }
    }
}
