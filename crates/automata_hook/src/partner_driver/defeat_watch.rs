use automata_game::defeat::{OtherPlayerHold, hold_cause};
use automata_game::partner::{BODY_HEALTH_OFFSET, BODY_STATE_OFFSET};
use automata_proto::PlayerState;
use nc_hook::diagnostics::log_line;

use crate::partner_driver::driver_state::PartnerDriver;

impl PartnerDriver {
    /// Logs the hero's health and each step of the game's defeat flow; true while that flow runs.
    pub(super) fn watch_health_and_defeat(
        &mut self,
        now_ms: u64,
        hero_body: Option<usize>,
    ) -> bool {
        let calls = self.calls;
        let int = |body: usize, offset| calls.read_i32_at(calls.body_field(body, offset)).ok();
        if let Some(body) = hero_body
            && let (Some(health), Some(max_health)) =
                (int(body, BODY_HEALTH_OFFSET), calls.read_max_health(body))
            && let Some(line) = self.hero_health_log.observe(health, max_health)
        {
            log_line(&line);
        }
        let Some(flow) = calls.read_defeat_flow() else {
            if !self.logged.defeat_flow_unreadable {
                self.logged.defeat_flow_unreadable = true;
                log_line(
                    "defeat: the game's defeat flow cannot be read, nothing about it is logged",
                );
            }
            return false;
        };
        let hero_state = hero_body.and_then(|body| int(body, BODY_STATE_OFFSET));
        let lines =
            self.defeat_flow_log
                .observe(now_ms, flow, hero_state, calls.read_restore_point());
        for line in lines {
            log_line(&line);
        }
        flow.is_active()
    }

    pub(super) fn note_hold(
        &mut self,
        now_ms: u64,
        both_free: bool,
        defeat_flow_runs: bool,
        newest_remote: Option<PlayerState>,
    ) {
        let cause = hold_cause(
            self.local_not_free,
            defeat_flow_runs,
            newest_remote.map(|remote| OtherPlayerHold {
                has_a_hero: !remote.load.without_body,
                free: remote.free,
                health: remote.health,
                state: remote.state,
            }),
        );
        if let Some(line) = self.hold_log.observe(now_ms, both_free, cause) {
            log_line(&line);
        }
    }
}
