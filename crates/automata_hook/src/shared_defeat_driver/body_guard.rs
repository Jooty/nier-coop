use automata_game::defeat::LOWEST_HEALTH_KEPT_ON_THE_PARTNER_BODY;
use automata_game::partner::BODY_HEALTH_OFFSET;
use automata_game::shared_defeat::{
    PARTNER_BODY_GUARD_LINES_LOGGED_AT_MOST, PartnerBodyGuardAct, partner_body_guard_act,
    partner_body_guard_line,
};
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::partner_driver::held_partner_body;

/// Keeps the held partner body out of the two paths that ask for this copy's defeat flow
/// without testing whose body it is; game thread only.
#[derive(Debug)]
pub(super) struct BodyGuard {
    lines: u32,
}

impl BodyGuard {
    pub(super) const fn new() -> Self {
        Self { lines: 0 }
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
        if self.lines < PARTNER_BODY_GUARD_LINES_LOGGED_AT_MOST
            && let Some(line) = partner_body_guard_line(act, reading, written)
        {
            self.lines += 1;
            log_line(&line);
        }
    }
}
