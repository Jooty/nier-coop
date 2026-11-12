use std::collections::{BTreeSet, HashMap, HashSet};
use std::time::Instant;

use automata_game::build::variant_of_image_size;
use automata_game::enemies::{
    Census, DefeatWatch, ENEMY_CENSUS_INTERVAL_MS, ENTITY_TABLE_IS_READ_EVERY_MS, EnemyHit,
    EnemyId, EnemySighting, GeneratorCensus, GuestHealthStep, HitLedger, HostSpawns,
    MOST_UNPLACED_ENEMIES_NAMED_IN_ONE_LINE, MismatchWatch, RosterAssembly, RosterPacer,
    amount_in_the_hosts_scale, distinct_sightings, guest_health_step, roster_mismatches,
};
use automata_game::enemy_lead::HOLD_LIFTS_AFTER_THE_HOSTS_SILENCE_MS;
use automata_game::save_redirect::SaveRole;
use automata_proto::Message;
use nc_hook::build_gate::VerifiedBuild;
use nc_hook::diagnostics::log_line;
use nc_hook::net::{NetState, NetThread};

use crate::enemies_driver::driver_state::{CensusSummary, DRIVER, EnemiesDriver};
use crate::enemies_driver::lead_state::LeadState;
use crate::enemies_driver::{
    CENSUS_SUMMARY_LINE_INTERVAL_MS, MISMATCH_LINES_LOGGED_AT_MOST, SHARE_LINES_LOGGED_AT_MOST,
    SPAWN_LINES_LOGGED_AT_MOST,
};
use crate::game_calls::{EnemyReading, GameCalls, PlacedEnemies};
use crate::game_hooks::{
    SharedEnemy, install_enemy_ai_hold, install_enemy_hits, install_generator_spawns,
    set_shared_enemies, take_reported_enemy_hits, while_a_heard_hit_is_applied,
};
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
        self.follow_the_lead(now_ms);
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
        let mut readings = placed.readings.clone();
        readings.extend(self.share_spawns(net, now_ms, connected));
        let all: Vec<EnemySighting> = readings.iter().map(|reading| reading.sighting).collect();
        let (roster, doubled) = distinct_sightings(&all);
        log_all(self.census.note(&roster, &doubled));
        self.log_summary(now_ms, &placed);
        self.log_unplaced_enemies(now_ms, &placed);
        if !connected {
            set_shared_enemies(HashMap::new());
            self.stop_leading();
            return;
        }
        if self.pacer.is_due(now_ms, &roster) {
            for part in self.pacer.parts_to_send(now_ms, &roster) {
                net.send_relayed(Message::EnemyRoster(part));
            }
        }
        self.log_mismatches(now_ms, &roster);
        if self.health_is_shared {
            let mine = self.enemies_that_can_be_shared(&readings, &roster);
            self.share_health(net, now_ms, &mine, &roster);
            self.lead_enemies(net, now_ms, &mine);
        }
    }

    fn note_link_change(&mut self, connected: bool) {
        self.was_connected = connected;
        self.pacer.forget();
        self.assembly.forget();
        self.other_roster = None;
        self.mismatches.forget();
        self.hits_heard.clear();
        self.own_hit_at_ms.clear();
        self.defeats_are_told_again = connected;
        if self.role == SaveRole::Guest {
            self.defeats.forget();
        }
        self.note_link_change_for_spawns(connected);
        self.stop_leading();
        log_line(if connected {
            "enemies: linked, this copy's list of placed enemies goes to the other copy"
        } else {
            "enemies: not linked, nothing is told or compared and every hit is the game's own"
        });
    }

    fn take_messages(&mut self, net: &NetThread<AutomataNet>) {
        let enemy_messages = net.take_relayed(|message| {
            matches!(
                message,
                Message::EnemyRoster(_)
                    | Message::EnemyHits(_)
                    | Message::EnemyDefeated(_)
                    | Message::EnemySpawned(_)
                    | Message::EnemyGone(_)
                    | Message::GeneratorStates(_)
                    | Message::EnemyLead(_)
            )
        });
        // Taken oldest first in one go: a defeat and a later list showing the enemy alive again
        // must be heard in the order the host said them.
        for message in enemy_messages {
            match message {
                Message::EnemyRoster(part) => {
                    if let Some(roster) = self.assembly.receive(part) {
                        self.hear_roster(roster);
                    }
                }
                Message::EnemyHits(hits) if self.role == SaveRole::Host => {
                    self.hits_heard.extend(hits.hits);
                }
                Message::EnemyDefeated(id) if self.role == SaveRole::Guest => {
                    self.defeats.hear(id);
                    self.hear_defeat_of_a_spawn(id);
                    self.log_share_line(format!(
                        "enemies: the host says {} is defeated",
                        id.label()
                    ));
                }
                Message::EnemySpawned(spawn) if self.role == SaveRole::Guest => {
                    self.hear_spawn(spawn);
                }
                Message::EnemyGone(id) if self.role == SaveRole::Guest => {
                    self.hear_gone(id);
                }
                Message::GeneratorStates(states) if self.role == SaveRole::Guest => {
                    self.hear_generator_states(&states.states);
                }
                Message::EnemyLead(lead) if self.role == SaveRole::Guest => {
                    self.hear_lead(&lead);
                }
                _ => {}
            }
        }
    }

    fn hear_roster(&mut self, roster: Vec<EnemySighting>) {
        if self.other_roster.is_none() {
            log_line(&format!(
                "enemies: first list from the other copy ({} placed enemies)",
                roster.len()
            ));
        }
        if self.role == SaveRole::Guest {
            self.defeats.forget_the_ones_alive_in(&roster);
        }
        self.other_roster = Some(roster);
    }

    fn enemies_that_can_be_shared(
        &mut self,
        readings: &[EnemyReading],
        roster: &[EnemySighting],
    ) -> Vec<EnemyReading> {
        let distinct: HashSet<EnemyId> = roster.iter().map(|sighting| sighting.id).collect();
        let mut mine = Vec::new();
        for reading in readings
            .iter()
            .filter(|reading| distinct.contains(&reading.sighting.id))
        {
            if reading.uses_the_base_damage_code {
                mine.push(*reading);
            } else if self
                .objects_with_their_own_damage_code
                .insert(reading.sighting.object_id)
            {
                self.log_share_line(format!(
                    "enemies: object {:#X} has its own damage code, its health and defeat stay each copy's own",
                    reading.sighting.object_id
                ));
            }
        }
        mine
    }

    fn share_health(
        &mut self,
        net: &NetThread<AutomataNet>,
        now_ms: u64,
        mine: &[EnemyReading],
        roster: &[EnemySighting],
    ) {
        let theirs: HashMap<EnemyId, EnemySighting> = self
            .other_roster
            .iter()
            .flatten()
            .map(|sighting| (sighting.id, *sighting))
            .collect();
        set_shared_enemies(
            mine.iter()
                .filter(|reading| {
                    theirs
                        .get(&reading.sighting.id)
                        .is_some_and(|other| other.object_id == reading.sighting.object_id)
                })
                .map(|reading| {
                    (
                        reading.body,
                        SharedEnemy {
                            id: reading.sighting.id,
                            handle: reading.handle,
                            max_health: reading.sighting.max_health,
                        },
                    )
                })
                .collect(),
        );
        match self.role {
            SaveRole::Host => {
                self.apply_the_guests_hits(mine);
                self.tell_defeats(net, roster);
            }
            SaveRole::Guest => {
                self.send_own_hits(net, now_ms);
                self.follow_the_host(now_ms, mine, &theirs);
            }
        }
    }

    fn apply_the_guests_hits(&mut self, mine: &[EnemyReading]) {
        for hit in core::mem::take(&mut self.hits_heard) {
            let Some(reading) = mine.iter().find(|reading| reading.sighting.id == hit.id) else {
                self.log_share_line(format!(
                    "enemies: the guest hit {}, which this copy does not hold as a shared enemy; the hit is dropped",
                    hit.id.label()
                ));
                continue;
            };
            self.apply_one_hit(reading, hit);
        }
    }

    #[expect(
        unsafe_code,
        reason = "calls the game's flat damage and defeat code on an enemy read this tick"
    )]
    fn apply_one_hit(&mut self, reading: &EnemyReading, hit: EnemyHit) {
        let Some(before) = self
            .calls
            .read_enemy_health(reading.body)
            .filter(|health| *health >= 1)
        else {
            return;
        };
        let amount =
            amount_in_the_hosts_scale(hit.amount, hit.max_health, reading.sighting.max_health);
        // SAFETY: the body was resolved from its live handle this tick on the game thread, is an
        // enemy by its object id, and its vtable holds the base damage functions.
        while_a_heard_hit_is_applied(|| unsafe {
            self.calls.apply_flat_damage(reading.body, amount);
        });
        let after = self.calls.read_enemy_health(reading.body).unwrap_or(before);
        let mut line = format!(
            "enemies: the guest's hit of {} (as {amount} here) on {}: health {before} -> {after}",
            hit.amount,
            hit.id.label()
        );
        if after < 1 {
            // SAFETY: as above; the flat damage code just left this enemy without health, and
            // outside the game's hit loop nothing else starts its defeat.
            let started = unsafe { self.calls.force_enemy_defeat(reading.body) };
            line.push_str(if started {
                ", defeat started with the game's forced defeat state"
            } else {
                ", the defeat could not be started"
            });
        }
        self.log_share_line(line);
    }

    fn tell_defeats(&mut self, net: &NetThread<AutomataNet>, roster: &[EnemySighting]) {
        let new = self.defeats.newly_defeated(roster);
        if core::mem::take(&mut self.defeats_are_told_again) {
            let held = self.defeats.all();
            for id in &held {
                net.send_relayed(Message::EnemyDefeated(*id));
            }
            if !held.is_empty() {
                self.log_share_line(format!(
                    "enemies: told the newly linked guest the {} enemies defeated here so far",
                    held.len()
                ));
            }
            return;
        }
        for id in new {
            net.send_relayed(Message::EnemyDefeated(id));
            self.log_share_line(format!(
                "enemies: {} is defeated here, told to the guest",
                id.label()
            ));
        }
    }

    fn send_own_hits(&mut self, net: &NetThread<AutomataNet>, now_ms: u64) {
        for hit in take_reported_enemy_hits() {
            self.own_hits.add(hit.id, hit.amount, hit.max_health);
            self.own_hit_at_ms.insert(hit.id, now_ms);
        }
        for hits in self.own_hits.take_messages() {
            net.send_relayed(Message::EnemyHits(hits));
        }
    }

    #[expect(
        unsafe_code,
        reason = "calls the game's defeat code on an enemy read this tick"
    )]
    fn follow_the_host(
        &mut self,
        now_ms: u64,
        mine: &[EnemyReading],
        theirs: &HashMap<EnemyId, EnemySighting>,
    ) {
        let alive_handles: HashSet<u32> = mine.iter().map(|reading| reading.handle).collect();
        self.handles_defeated_here
            .retain(|handle| alive_handles.contains(handle));
        let ids: HashSet<EnemyId> = mine.iter().map(|reading| reading.sighting.id).collect();
        self.own_hit_at_ms.retain(|id, _| ids.contains(id));
        for reading in mine {
            let id = reading.sighting.id;
            let step = guest_health_step(
                &reading.sighting,
                theirs.get(&id),
                self.defeats.holds(id),
                self.own_hit_at_ms
                    .get(&id)
                    .map(|at_ms| now_ms.saturating_sub(*at_ms)),
            );
            match step {
                GuestHealthStep::Leave => {}
                GuestHealthStep::Write(health) => {
                    self.calls.write_enemy_health(reading.body, health);
                }
                GuestHealthStep::Defeat => {
                    if !self.handles_defeated_here.insert(reading.handle) {
                        continue;
                    }
                    // SAFETY: the body was resolved from its live handle this tick on the game
                    // thread, is an enemy by its object id, and its vtable holds the base damage
                    // functions.
                    let started = unsafe { self.calls.force_enemy_defeat(reading.body) };
                    self.log_share_line(format!(
                        "enemies: {} is defeated on the host, {} here (health was {})",
                        id.label(),
                        if started {
                            "defeat started with the game's forced defeat state"
                        } else {
                            "the defeat could not be started"
                        },
                        reading.sighting.health
                    ));
                }
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
            objects_without_an_enemys_health: placed.objects_without_an_enemys_health,
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
                "enemies: {} entity layouts lists, {} placed enemies alive or falling, {} of them with the base damage code, {} whose own id field differs from the entry's; {} more placed objects in the enemy id range hold no enemy's health and are left out",
                summary.lists,
                summary.enemies,
                summary.with_the_base_damage_code,
                summary.without_the_entry_id_on_the_body,
                summary.objects_without_an_enemys_health
            ));
        }
    }

    fn log_unplaced_enemies(&mut self, now_ms: u64, placed: &PlacedEnemies) {
        if !local_in_play()
            || self
                .entity_table_read_at_ms
                .is_some_and(|at_ms| now_ms.saturating_sub(at_ms) < ENTITY_TABLE_IS_READ_EVERY_MS)
        {
            return;
        }
        self.entity_table_read_at_ms = Some(now_ms);
        let handles: Vec<u32> = placed
            .readings
            .iter()
            .map(|reading| reading.handle)
            .collect();
        let (count, first) = self
            .calls
            .read_unplaced_enemies(&handles, MOST_UNPLACED_ENEMIES_NAMED_IN_ONE_LINE);
        let counts = (count, handles.len());
        if self.unplaced_logged == Some(counts) || (count == 0 && self.unplaced_logged.is_none()) {
            return;
        }
        self.unplaced_logged = Some(counts);
        let named: Vec<String> = first
            .iter()
            .map(|enemy| {
                format!(
                    "object {:#X} named {:?} with set id {:#X} and set word {:#X}",
                    enemy.object_id, enemy.name, enemy.set_id, enemy.set_word
                )
            })
            .collect();
        self.log_share_line(format!(
            "enemies: the entity table holds {count} enemies no layout entry holds (and {} placed ones); the first: {}",
            handles.len(),
            named.join("; ")
        ));
    }

    fn log_mismatches(&mut self, now_ms: u64, roster: &[EnemySighting]) {
        let Some(other) = &self.other_roster else {
            return;
        };
        // Until the host owns the spawns each copy's generators make their own enemies: only
        // placed enemies can be compared then.
        let spawns_follow_the_host = self.spawns_follow_the_host;
        let compared =
            |sighting: &&EnemySighting| spawns_follow_the_host || !sighting.id.is_generated();
        let here: Vec<EnemySighting> = roster.iter().filter(compared).copied().collect();
        let there: Vec<EnemySighting> = other.iter().filter(compared).copied().collect();
        for line in self
            .mismatches
            .note(now_ms, &roster_mismatches(&here, &there))
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

    fn log_share_line(&mut self, line: String) {
        if self.share_lines_left == 0 {
            return;
        }
        self.share_lines_left -= 1;
        log_line(&line);
        if self.share_lines_left == 0 {
            log_line("enemies: no more hit and defeat lines this session");
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
    let health_is_shared =
        settings.enemy_share.shares_health() && install_enemy_hits(calls, settings.role);
    let generators_are_hooked = install_generator_spawns(calls);
    let lead = lead_state_installed(calls, health_is_shared);
    let spawns_follow_the_host =
        generators_are_hooked && settings.enemy_share.lets_the_host_own_the_spawns();
    log_line(&format!(
        "enemies: placed enemies are listed by the ids of the game's own layout data and compared with the other copy's (shared_enemies {}, this copy is the {}); health and defeat follow the host: {health_is_shared}; generators are watched: {generators_are_hooked}; while linked the host's generators decide every spawn and the guest's are held: {spawns_follow_the_host} (every step of it is a guess until it has run)",
        settings.enemy_share.label(),
        settings.role.folder_name()
    ));
    let Ok(mut guard) = DRIVER.lock() else {
        return;
    };
    *guard = Some(EnemiesDriver {
        calls,
        role: settings.role,
        health_is_shared,
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
        defeats: DefeatWatch::default(),
        defeats_are_told_again: false,
        hits_heard: Vec::new(),
        own_hits: HitLedger::default(),
        own_hit_at_ms: HashMap::new(),
        handles_defeated_here: HashSet::new(),
        objects_with_their_own_damage_code: HashSet::new(),
        share_lines_left: SHARE_LINES_LOGGED_AT_MOST,
        entity_table_read_at_ms: None,
        unplaced_logged: None,
        generators_are_hooked,
        spawns_follow_the_host,
        host_spawns: HostSpawns::default(),
        generator_census: GeneratorCensus::default(),
        generator_states: Vec::new(),
        generators_read_at_ms: None,
        generators_are_held: false,
        living_spawns_are_told_again: false,
        generators_left_to_the_game: BTreeSet::new(),
        spawn_lines_left: SPAWN_LINES_LOGGED_AT_MOST,
        generators_live_here: BTreeSet::new(),
        spawns_waiting_without_a_generator_since_ms: HashMap::new(),
        enemy_reads_asked: HashMap::new(),
        lead,
    });
}

fn lead_state_installed(calls: GameCalls, health_is_shared: bool) -> LeadState {
    if !health_is_shared {
        return LeadState::default();
    }
    let (states_are_hooked, targets_are_hooked) = install_enemy_ai_hold(calls);
    log_line(&format!(
        "enemies: while linked the host leads every shared enemy: place, facing and state ten times a second, the guest's own state starts held: {states_are_hooked}, targets picked on the host: {targets_are_hooked}; a hold lifts after {HOLD_LIFTS_AFTER_THE_HOSTS_SILENCE_MS} ms without the host's word (every step of it is a guess until it has run)"
    ));
    LeadState {
        active: true,
        states_are_hooked,
        targets_are_hooked,
        ..LeadState::default()
    }
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

pub(super) fn log_all(lines: Vec<String>) {
    for line in lines {
        log_line(&line);
    }
}
