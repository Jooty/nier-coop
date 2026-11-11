use automata_game::address::DEFEAT_LEFT_BODY_RECORD_FLAG_VALUE;
use automata_game::defeat::{
    DefeatFlow, NO_LEFT_BODY_RECORD, OtherPlayerHold, health_kept_on_the_partner_body, hold_cause,
    left_body_record_is_dropped,
};
use automata_game::partner::{BODY_BASE_HEALTH_OFFSET, BODY_HEALTH_OFFSET, BODY_STATE_OFFSET};
use automata_proto::PlayerState;
use nc_hook::diagnostics::log_line;

use crate::partner_driver::driver_state::PartnerDriver;

impl PartnerDriver {
    /// Logs the hero's health and each step of the game's defeat flow; true while that flow runs.
    pub(super) fn watch_health_and_defeat(
        &mut self,
        now_ms: u64,
        hero_body: Option<usize>,
        session_linked: bool,
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
        self.drop_the_left_body_record(session_linked, flow);
        flow.is_active()
    }

    fn drop_the_left_body_record(&mut self, session_linked: bool, flow: DefeatFlow) {
        if !left_body_record_is_dropped(session_linked, flow) {
            return;
        }
        let calls = self.calls;
        if calls.read_i32(DEFEAT_LEFT_BODY_RECORD_FLAG_VALUE) == Ok(NO_LEFT_BODY_RECORD) {
            return;
        }
        let written = calls
            .write_i32(DEFEAT_LEFT_BODY_RECORD_FLAG_VALUE, NO_LEFT_BODY_RECORD)
            .is_ok();
        log_line(&format!(
            "defeat: dropped the game's record of the body left behind before its flow read it (written {written}; flow at {}): in a linked session the flow shows no screen for it and sends nothing out",
            flow.describe()
        ));
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

    pub(super) fn note_the_other_players_health(&mut self, newest_remote: Option<PlayerState>) {
        if let Some(remote) = newest_remote.filter(|remote| !remote.load.without_body)
            && let Some(line) = self
                .other_player_health_log
                .observe(remote.health, remote.max_health)
        {
            log_line(&line);
        }
    }

    /// Writes the other player's health into the held body, never below 1.
    pub(super) fn show_the_owners_health(&mut self, body: usize, remote: &PlayerState) {
        let calls = self.calls;
        let Some(body_max_health) = calls.read_max_health(body) else {
            return;
        };
        let wanted = health_kept_on_the_partner_body(remote.health, body_max_health);
        let mut written = false;
        for offset in [BODY_HEALTH_OFFSET, BODY_BASE_HEALTH_OFFSET] {
            let address = calls.body_field(body, offset);
            if calls.read_i32_at(address).is_ok_and(|now| now != wanted) {
                written |= calls.write_i32_at(address, wanted).is_ok();
            }
        }
        if written && !self.logged.owners_health_shown {
            self.logged.owners_health_shown = true;
            log_line(&format!(
                "partner: first write of the other player's health into the body ({wanted}; the other player has {} of {}, the body can have {body_max_health}); the body keeps at least 1 here, its defeat is the other game's",
                remote.health, remote.max_health
            ));
        }
    }
}
