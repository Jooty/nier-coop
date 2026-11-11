use std::time::Instant;

use automata_game::build::variant_of_image_size;
use automata_game::enemies::{
    Census, ENEMY_CENSUS_INTERVAL_MS, EnemySighting, MismatchWatch, RosterAssembly, RosterPacer,
    distinct_sightings, roster_mismatches,
};
use automata_proto::Message;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::{NetState, NetThread};

use crate::enemies_driver::driver_state::{CensusSummary, DRIVER, EnemiesDriver};
use crate::enemies_driver::{CENSUS_SUMMARY_LINE_INTERVAL_MS, MISMATCH_LINES_LOGGED_AT_MOST};
use crate::game_calls::{GameCalls, PlacedEnemies};
use crate::net::{AutomataNet, handle, local_in_play};
use crate::start_setting::GameSettings;

impl EnemiesDriver {
    fn tick(&mut self, net: &NetThread<AutomataNet>) {
        let now_ms = self.started.elapsed().as_millis() as u64;
        let connected = net.update(|shared| shared.state == NetState::Connected);
        if connected != self.was_connected {
            self.note_link_change(connected);
        }
        self.take_messages(net);
        if self
            .last_census_ms
            .is_some_and(|at_ms| now_ms.saturating_sub(at_ms) < ENEMY_CENSUS_INTERVAL_MS)
        {
            return;
        }
        self.last_census_ms = Some(now_ms);
        let placed = if local_in_play() {
            self.calls.read_placed_enemies()
        } else {
            PlacedEnemies::default()
        };
        let all: Vec<EnemySighting> = placed
            .readings
            .iter()
            .map(|reading| reading.sighting)
            .collect();
        let (roster, doubled) = distinct_sightings(&all);
        log_all(self.census.note(&roster, &doubled));
        self.log_summary(now_ms, &placed);
        if !connected {
            return;
        }
        if self.pacer.is_due(now_ms, &roster) {
            for part in self.pacer.parts_to_send(now_ms, &roster) {
                net.send_relayed(Message::EnemyRoster(part));
            }
        }
        self.log_mismatches(now_ms, &roster);
    }

    fn note_link_change(&mut self, connected: bool) {
        self.was_connected = connected;
        self.pacer.forget();
        self.assembly.forget();
        self.other_roster = None;
        self.mismatches.forget();
        log_line(if connected {
            "enemies: linked, this copy's list of placed enemies goes to the other copy"
        } else {
            "enemies: not linked, nothing is told or compared"
        });
    }

    fn take_messages(&mut self, net: &NetThread<AutomataNet>) {
        for message in net.take_relayed(|message| matches!(message, Message::EnemyRoster(_))) {
            if let Message::EnemyRoster(part) = message
                && let Some(roster) = self.assembly.receive(part)
            {
                if self.other_roster.is_none() {
                    log_line(&format!(
                        "enemies: first list from the other copy ({} placed enemies)",
                        roster.len()
                    ));
                }
                self.other_roster = Some(roster);
            }
        }
    }

    fn log_summary(&mut self, now_ms: u64, placed: &PlacedEnemies) {
        let summary = CensusSummary {
            lists: placed.lists,
            enemies: placed.readings.len(),
            with_the_base_damage_code: placed
                .readings
                .iter()
                .filter(|reading| reading.uses_the_base_damage_code)
                .count(),
            without_the_entry_id_on_the_body: placed
                .readings
                .iter()
                .filter(|reading| !reading.body_carries_the_entry_id)
                .count(),
        };
        let due = match self.summary_logged {
            None => summary != CensusSummary::default(),
            Some((logged, at_ms)) => {
                logged != summary && now_ms.saturating_sub(at_ms) >= CENSUS_SUMMARY_LINE_INTERVAL_MS
            }
        };
        if due {
            self.summary_logged = Some((summary, now_ms));
            log_line(&format!(
                "enemies: {} entity layouts lists, {} placed enemies alive or falling, {} of them with the base damage code, {} whose own id field differs from the entry's",
                summary.lists,
                summary.enemies,
                summary.with_the_base_damage_code,
                summary.without_the_entry_id_on_the_body
            ));
        }
    }

    fn log_mismatches(&mut self, now_ms: u64, roster: &[EnemySighting]) {
        let Some(other) = &self.other_roster else {
            return;
        };
        for line in self
            .mismatches
            .note(now_ms, &roster_mismatches(roster, other))
        {
            if self.mismatch_lines_left == 0 {
                return;
            }
            self.mismatch_lines_left -= 1;
            log_line(&line);
            if self.mismatch_lines_left == 0 {
                log_line("enemies: no more mismatch lines this session");
            }
        }
    }
}

pub fn install(build: VerifiedBuild, settings: &GameSettings) {
    let Some(variant) = variant_of_image_size(build.size_of_image()) else {
        return;
    };
    if settings.link_port.is_none() || !settings.enemy_share.lists_enemies() {
        log_line("enemies: off (no link port, or the setting shared_enemies is off)");
        return;
    }
    let calls = GameCalls::new(build, variant);
    log_line(&format!(
        "enemies: placed enemies are listed by the ids of the game's own layout data and compared with the other copy's (shared_enemies {}, this copy is the {})",
        settings.enemy_share.label(),
        settings.role.folder_name()
    ));
    let Ok(mut guard) = DRIVER.lock() else {
        return;
    };
    *guard = Some(EnemiesDriver {
        calls,
        started: Instant::now(),
        last_census_ms: None,
        census: Census::default(),
        pacer: RosterPacer::default(),
        assembly: RosterAssembly::default(),
        other_roster: None,
        mismatches: MismatchWatch::default(),
        mismatch_lines_left: MISMATCH_LINES_LOGGED_AT_MOST,
        was_connected: false,
        summary_logged: None,
    });
}

pub fn tick() {
    let Ok(mut guard) = DRIVER.try_lock() else {
        return;
    };
    let (Some(driver), Some(net)) = (guard.as_mut(), handle()) else {
        return;
    };
    driver.tick(net);
}

fn log_all(lines: Vec<String>) {
    for line in lines {
        log_line(&line);
    }
}
