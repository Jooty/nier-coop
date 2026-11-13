use std::collections::BTreeSet;

use automata_game::enemies::{
    EnemyId, EnemySpawn, GENERATORS_ARE_READ_EVERY_MS, GeneratorState, LiveGenerators,
    SPAWN_WITHOUT_A_GENERATOR_IS_MADE_WITHIN_METRES, SPAWN_WITHOUT_A_GENERATOR_WAITS_MS,
    generators_with_a_shared_id, leave_test_rule,
};
use automata_game::enemies::{GeneratorCountReading, GeneratorHeader, GeneratorRunCounts};
use automata_game::save_redirect::SaveRole;
use automata_game::start::route_flag_hash_held;
use automata_proto::Message;
use nc_hook::diagnostics::log_line;
use nc_hook::net::NetThread;

use crate::enemies_driver::MOST_GENERATOR_STATES_NAMED_IN_ONE_LINE;
use crate::enemies_driver::driver::log_all;
use crate::enemies_driver::driver_state::EnemiesDriver;
use crate::game_calls::EnemyReading;
use crate::game_hooks::{
    set_generators_left_to_the_game, set_guest_generators_held, set_leave_test_rule,
    take_observed_spawns, with_guest_generators,
};
use crate::net::{AutomataNet, local_in_play};

impl EnemiesDriver {
    /// Tells or follows the generators' spawns and returns the generators' enemies that have an
    /// id on both copies, for the lists and the health share.
    pub(super) fn share_spawns(
        &mut self,
        net: &NetThread<AutomataNet>,
        now_ms: u64,
        connected: bool,
    ) -> Vec<EnemyReading> {
        if !self.generators_are_hooked {
            return Vec::new();
        }
        if self.read_generators(now_ms) {
            self.tell_or_compare_the_live_generators(net, now_ms, connected);
        }
        self.hold_or_release_the_guests_generators(connected);
        let observed = take_observed_spawns();
        match self.role {
            SaveRole::Host => {
                for spawn in observed {
                    if self.generators_left_to_the_game.contains(&spawn.generator) {
                        self.log_spawn_line(format!(
                            "enemies: a generator with the id {:#X}, which names no single generator, made object {:#X}; it stays this copy's own",
                            spawn.generator, spawn.object_id
                        ));
                        continue;
                    }
                    let spawn = self.host_spawns.note(spawn);
                    if connected {
                        net.send_relayed(Message::EnemySpawned(spawn));
                    }
                    self.log_spawn_line(format!(
                        "enemies: spawn decided by the host: {}{}",
                        spawn.label(),
                        if connected {
                            ", told to the guest"
                        } else {
                            ", no guest is linked"
                        }
                    ));
                }
                if connected {
                    self.tell_the_guest_what_stands(net);
                }
                self.read_the_hosts_spawns(net, connected)
            }
            SaveRole::Guest => {
                for spawn in observed {
                    self.log_spawn_line(format!(
                        "enemies: this copy's own generator {:#X} made object {:#X} from set {} (its own decision: generators are held only while linked with shared_enemies on)",
                        spawn.generator, spawn.object_id, spawn.set_index
                    ));
                }
                if self.generators_are_held {
                    self.make_the_spawns_without_a_generator_here(now_ms);
                }
                self.release_the_enemy_reads_no_longer_needed();
                self.read_the_enemies_made_from_the_hosts_spawns(now_ms)
            }
        }
    }

    pub(super) fn note_link_change_for_spawns(&mut self, connected: bool) {
        match self.role {
            SaveRole::Host => {
                self.host_spawns.forget_what_the_guest_was_told();
                self.living_spawns_are_told_again = connected;
                self.live_generators_pacer.forget();
            }
            SaveRole::Guest => {
                self.hosts_word_on_enemies_at_ms = None;
                self.leftovers_taken_away.clear();
                self.live_generators_assembly.forget();
                self.generator_set_comparison.forget();
                self.generator_counts.forget();
                self.remove_the_enemies_made_without_a_generator();
                with_guest_generators(|guest| guest.forget());
                self.spawns_waiting_without_a_generator_since_ms.clear();
            }
        }
    }

    pub(super) fn hear_spawn(&mut self, spawn: EnemySpawn) {
        let lines = with_guest_generators(|guest| guest.hear_spawn(spawn)).unwrap_or_default();
        for line in lines {
            self.log_spawn_line(line);
        }
        if !self.spawns_follow_the_host {
            self.log_spawn_line(format!(
                "enemies: {} is not made here: shared_enemies is not on",
                spawn.id().label()
            ));
        }
    }

    pub(super) fn hear_defeat_of_a_spawn(&mut self, id: EnemyId) {
        if !id.is_generated() {
            return;
        }
        if let Some(line) = with_guest_generators(|guest| guest.hear_gone(id, true)).flatten() {
            self.log_spawn_line(line);
        }
    }

    /// Who ends a generator's run on this copy, renewed every tick so it lapses when the driver
    /// stops.
    pub(super) fn renew_the_leave_test_rule(&mut self, now_ms: u64, connected: bool) {
        if !self.leave_test_is_hooked {
            return;
        }
        let rule = leave_test_rule(
            self.role == SaveRole::Host,
            self.spawns_follow_the_host,
            connected,
            self.generators_are_held,
            self.hosts_word_on_enemies_at_ms
                .map(|at_ms| now_ms.saturating_sub(at_ms)),
        );
        if let Some(line) = set_leave_test_rule(rule, &self.generators_left_to_the_game) {
            log_line(&line);
        }
    }

    pub(super) fn hear_gone(&mut self, id: EnemyId, now_ms: u64) {
        let defeated = self.defeats.holds(id);
        self.log_spawn_line(format!(
            "enemies: the host says {} is gone from its game ({})",
            id.label(),
            if defeated {
                "after its defeat"
            } else {
                "without a defeat: taken away"
            }
        ));
        if let Some(line) =
            with_guest_generators(|guest| guest.hear_gone_from_the_hosts_game(id, defeated, now_ms))
                .flatten()
        {
            self.log_spawn_line(line);
        }
    }

    pub(super) fn hear_generator_states(&mut self, states: &[GeneratorState]) {
        with_guest_generators(|guest| guest.hear_states(states));
        self.log_spawn_line(format!(
            "enemies: the host says where {} of its generators stand: {}",
            states.len(),
            first_states_named(states)
        ));
    }

    fn read_generators(&mut self, now_ms: u64) -> bool {
        if self
            .generators_read_at_ms
            .is_some_and(|at_ms| now_ms.saturating_sub(at_ms) < GENERATORS_ARE_READ_EVERY_MS)
        {
            return false;
        }
        self.generators_read_at_ms = Some(now_ms);
        let in_play = local_in_play();
        let generators: Vec<_> = if in_play {
            self.calls
                .read_generators()
                .into_iter()
                .map(|(_, header)| header)
                .collect()
        } else {
            Vec::new()
        };
        self.route_hash_here = if in_play {
            route_flag_hash_held(|hash| self.calls.scene_flag_is_set(hash))
        } else {
            0
        };
        log_all(self.generator_census.note(&generators));
        let left_to_the_game = generators_with_a_shared_id(&generators);
        let tells_run_counts = self.counts_follow_the_host && self.role == SaveRole::Host;
        self.generator_states = generators
            .iter()
            .filter(|generator| !left_to_the_game.contains(&generator.action_id))
            .map(|generator| GeneratorState {
                generator: generator.action_id,
                phase: generator.phase(),
                spawned: u32::try_from(generator.spawned).unwrap_or(u32::MAX),
                counts: (tells_run_counts && !generator.phase().is_not_started())
                    .then(|| {
                        let slots = self.calls.read_generator_slots(generator);
                        GeneratorRunCounts::of_generator(generator, &slots)
                    })
                    .flatten(),
            })
            .collect();
        self.compare_the_generator_counts(now_ms, &generators, &left_to_the_game);
        self.generators_live_here = generators
            .iter()
            .map(|generator| generator.action_id)
            .collect();
        set_generators_left_to_the_game(left_to_the_game.clone());
        self.generators_left_to_the_game = left_to_the_game;
        true
    }

    fn compare_the_generator_counts(
        &mut self,
        now_ms: u64,
        generators: &[GeneratorHeader],
        left_to_the_game: &BTreeSet<u32>,
    ) {
        if !self.counts_follow_the_host || self.role != SaveRole::Guest || !self.generators_are_held
        {
            return;
        }
        let readings: Vec<_> = generators
            .iter()
            .filter(|generator| !left_to_the_game.contains(&generator.action_id))
            .map(|generator| {
                let slots = if generator.phase().is_not_started() {
                    Vec::new()
                } else {
                    self.calls.read_generator_slots(generator)
                };
                let reading = GeneratorCountReading::of_generator(generator, &slots);
                let host =
                    with_guest_generators(|guest| guest.host_state(generator.action_id)).flatten();
                (reading, host)
            })
            .collect();
        log_all(self.generator_counts.note(now_ms, &readings));
    }

    fn named_generators_live_here(&self) -> BTreeSet<u32> {
        self.generators_live_here
            .difference(&self.generators_left_to_the_game)
            .copied()
            .collect()
    }

    fn tell_or_compare_the_live_generators(
        &mut self,
        net: &NetThread<AutomataNet>,
        now_ms: u64,
        connected: bool,
    ) {
        if !connected {
            return;
        }
        let live = self.named_generators_live_here();
        match self.role {
            SaveRole::Host => {
                for part in
                    self.live_generators_pacer
                        .parts_to_send(now_ms, self.route_hash_here, &live)
                {
                    net.send_relayed(Message::LiveGenerators(part));
                }
            }
            SaveRole::Guest => {
                if let Some(line) =
                    self.generator_set_comparison
                        .note(now_ms, self.route_hash_here, &live)
                {
                    log_line(&line);
                }
            }
        }
    }

    pub(super) fn hear_live_generators(&mut self, part: LiveGenerators) {
        if let Some((route_hash, ids)) = self.live_generators_assembly.receive(part) {
            self.generator_set_comparison
                .hear_the_hosts(route_hash, ids);
        }
    }

    fn hold_or_release_the_guests_generators(&mut self, connected: bool) {
        let held = self.role == SaveRole::Guest && self.spawns_follow_the_host && connected;
        if held == self.generators_are_held {
            return;
        }
        self.generators_are_held = held;
        set_guest_generators_held(held);
        log_line(if held {
            "enemies: linked with shared_enemies on: this copy's generators are held from now, they make only what the host's generators decide"
        } else {
            "enemies: this copy's generators decide their own spawns again"
        });
    }

    fn tell_the_guest_what_stands(&mut self, net: &NetThread<AutomataNet>) {
        if core::mem::take(&mut self.living_spawns_are_told_again) {
            let standing = self.host_spawns.spawns_still_there();
            for spawn in &standing {
                net.send_relayed(Message::EnemySpawned(*spawn));
            }
            self.log_spawn_line(format!(
                "enemies: told the newly linked guest the {} generator spawns still there",
                standing.len()
            ));
        }
        for states in self.host_spawns.states_to_tell(&self.generator_states) {
            self.log_spawn_line(format!(
                "enemies: told the guest where {} generators stand: {}",
                states.states.len(),
                first_states_named(&states.states)
            ));
            net.send_relayed(Message::GeneratorStates(states));
        }
    }

    fn read_the_hosts_spawns(
        &mut self,
        net: &NetThread<AutomataNet>,
        connected: bool,
    ) -> Vec<EnemyReading> {
        let mut readings = Vec::new();
        for (id, handle) in self.host_spawns.living() {
            if self.calls.entity_of_handle(handle) == 0 {
                self.host_spawns.note_gone(id);
                if connected {
                    net.send_relayed(Message::EnemyGone(id));
                }
                self.log_spawn_line(format!(
                    "enemies: {} is gone from this game{}",
                    id.label(),
                    if connected { ", told to the guest" } else { "" }
                ));
                continue;
            }
            self.log_the_group_word_once(id, handle, "made by the host's own generator");
            readings.extend(
                self.calls
                    .read_generated_enemy(id, handle)
                    .filter(|reading| reading.reads_as_an_enemy),
            );
        }
        readings
    }

    fn log_the_group_word_once(&mut self, id: EnemyId, handle: u32, how: &str) {
        if !self.group_words_logged.insert(id) {
            return;
        }
        let word = self.calls.generator_group_word_of_handle(handle);
        self.log_spawn_line(format!(
            "enemies: group word of {} ({how}, handle {handle:#X}): {}",
            id.label(),
            word.label()
        ));
    }

    #[expect(
        unsafe_code,
        reason = "calls the game's entity terminate on an enemy this mod had the generator make"
    )]
    fn read_the_enemies_made_from_the_hosts_spawns(&mut self, now_ms: u64) -> Vec<EnemyReading> {
        if self.generators_are_held {
            let taken_away = with_guest_generators(|guest| guest.made_and_gone_on_the_host())
                .unwrap_or_default();
            for (id, handle) in taken_away {
                let entity = self.calls.entity_of_handle(handle);
                if entity == 0 {
                    continue;
                }
                // SAFETY: the entity was resolved from its live handle on the game thread just
                // now, and it is an enemy this copy's generator made from the host's spawn.
                unsafe { self.calls.terminate_entity(entity) };
                self.log_spawn_line(format!(
                    "enemies: {} was taken away on the host, removed here (handle {handle:#X})",
                    id.label()
                ));
            }
            self.take_away_the_defeated_enemies_that_outlive_the_hosts(now_ms);
        }
        let mut readings = Vec::new();
        let without_a_generator: Vec<EnemyId> =
            with_guest_generators(|guest| guest.made_without_a_generator())
                .unwrap_or_default()
                .into_iter()
                .map(|(id, _)| id)
                .collect();
        for (id, handle) in with_guest_generators(|guest| guest.made()).unwrap_or_default() {
            if self.calls.entity_of_handle(handle) == 0 {
                if let Some(line) =
                    with_guest_generators(|guest| guest.note_made_enemy_gone(id)).flatten()
                {
                    self.log_spawn_line(line);
                }
                continue;
            }
            self.log_the_group_word_once(
                id,
                handle,
                if without_a_generator.contains(&id) {
                    "made here without a generator, nothing writes the word"
                } else {
                    "made here by this copy's own generator"
                },
            );
            readings.extend(
                self.calls
                    .read_generated_enemy(id, handle)
                    .filter(|reading| reading.reads_as_an_enemy),
            );
        }
        readings
    }

    #[expect(
        unsafe_code,
        reason = "calls the game's entity terminate on an enemy the host defeated and no longer has"
    )]
    fn take_away_the_defeated_enemies_that_outlive_the_hosts(&mut self, now_ms: u64) {
        let outliving =
            with_guest_generators(|guest| guest.made_and_defeated_and_outliving_the_hosts(now_ms))
                .unwrap_or_default();
        self.leftovers_taken_away
            .retain(|handle| outliving.iter().any(|(_, outliving)| outliving == handle));
        for (id, handle) in outliving {
            let entity = self.calls.entity_of_handle(handle);
            if entity == 0 || !self.leftovers_taken_away.insert(handle) {
                continue;
            }
            let reading = self.calls.read_generated_enemy(id, handle);
            let health = reading.map(|reading| reading.sighting.health);
            let state = reading
                .and_then(|reading| self.calls.read_enemy_pose(reading.body))
                .map(|pose| pose.state);
            // SAFETY: the entity was resolved from its live handle on the game thread just now,
            // and it is an enemy this copy made from the host's spawn, defeated and gone there.
            unsafe { self.calls.terminate_entity(entity) };
            self.log_spawn_line(format!(
                "enemies: {} was defeated on the host and is gone there, but here it still stands (health {}, state {}); removed here (handle {handle:#X})",
                id.label(),
                health.map_or("unread".to_owned(), |health| health.to_string()),
                state.map_or("unread".to_owned(), |state| format!(
                    "{:#X} {} {} {}",
                    state[0], state[1], state[2], state[3]
                ))
            ));
        }
    }

    #[expect(
        unsafe_code,
        reason = "calls the game's spawn for an enemy whose data the game has read"
    )]
    fn make_the_spawns_without_a_generator_here(&mut self, now_ms: u64) {
        if !local_in_play() || self.generators_live_here.is_empty() {
            self.spawns_waiting_without_a_generator_since_ms.clear();
            return;
        }
        let hero = self.calls.current_hero();
        let hero_position = (hero != 0)
            .then(|| self.calls.body_position(hero))
            .flatten();
        let live = self.generators_live_here.clone();
        let waiting = with_guest_generators(|guest| guest.spawns_without_a_generator_here(&live))
            .unwrap_or_default();
        self.spawns_waiting_without_a_generator_since_ms
            .retain(|id, _| waiting.iter().any(|spawn| spawn.id() == *id));
        for spawn in waiting {
            let since_ms = *self
                .spawns_waiting_without_a_generator_since_ms
                .entry(spawn.id())
                .or_insert(now_ms);
            let near = hero_position.is_some_and(|hero| {
                let apart: f32 = (0..3)
                    .map(|axis| (hero[axis] - spawn.position[axis]).powi(2))
                    .sum();
                apart.sqrt() <= SPAWN_WITHOUT_A_GENERATOR_IS_MADE_WITHIN_METRES
            });
            if !near || now_ms.saturating_sub(since_ms) < SPAWN_WITHOUT_A_GENERATOR_WAITS_MS {
                continue;
            }
            let asked = self.calls.request_enemy_read(&spawn);
            self.enemy_reads_asked
                .insert(spawn.id(), (spawn.object_id, spawn.object_read_variant));
            with_guest_generators(|guest| guest.note_load_asked_without_a_generator(spawn.id()));
            self.log_spawn_line(format!(
                "enemies: mismatch: this copy has no generator {:#X}; {} is made without one, its data is asked for (taken: {asked})",
                spawn.generator,
                spawn.id().label()
            ));
        }
        let loading =
            with_guest_generators(|guest| guest.loading_without_a_generator()).unwrap_or_default();
        for spawn in loading {
            if !self.calls.enemy_read_is_complete(&spawn) {
                continue;
            }
            // SAFETY: the game just said the read of this enemy's data is complete, and this is
            // the game thread.
            let handle = unsafe { self.calls.spawn_enemy_without_a_generator(&spawn) };
            match handle {
                Some(handle) => {
                    with_guest_generators(|guest| {
                        guest.note_made_without_a_generator(spawn.id(), handle);
                    });
                    self.log_spawn_line(format!(
                        "enemies: made {} from the host's message with the game's spawn call, no generator (handle {handle:#X})",
                        spawn.label()
                    ));
                }
                None => {
                    with_guest_generators(|guest| guest.note_refused(spawn.id()));
                    self.log_spawn_line(format!(
                        "enemies: mismatch: the game's spawn call made nothing for {}",
                        spawn.label()
                    ));
                }
            }
        }
    }

    fn release_the_enemy_reads_no_longer_needed(&mut self) {
        let over: Vec<EnemyId> = self
            .enemy_reads_asked
            .keys()
            .filter(|id| !with_guest_generators(|guest| guest.holds(**id)).unwrap_or(false))
            .copied()
            .collect();
        for id in over {
            if let Some((object_id, variant)) = self.enemy_reads_asked.remove(&id) {
                self.calls.release_enemy_read(object_id, variant);
            }
        }
    }

    #[expect(
        unsafe_code,
        reason = "calls the game's entity terminate on enemies this mod spawned itself"
    )]
    fn remove_the_enemies_made_without_a_generator(&mut self) {
        let made =
            with_guest_generators(|guest| guest.made_without_a_generator()).unwrap_or_default();
        for (id, handle) in made {
            let entity = self.calls.entity_of_handle(handle);
            if entity == 0 {
                continue;
            }
            // SAFETY: the entity was resolved from its live handle on the game thread just now
            // and this mod spawned it; with the link gone nothing here looks after it.
            unsafe { self.calls.terminate_entity(entity) };
            self.log_spawn_line(format!(
                "enemies: the link is gone, {} made without a generator is removed",
                id.label()
            ));
        }
    }

    fn log_spawn_line(&mut self, line: String) {
        if self.spawn_lines_left == 0 {
            return;
        }
        self.spawn_lines_left -= 1;
        log_line(&line);
        if self.spawn_lines_left == 0 {
            log_line("enemies: no more spawn lines this session");
        }
    }
}

fn first_states_named(states: &[GeneratorState]) -> String {
    let named: Vec<String> = states
        .iter()
        .take(MOST_GENERATOR_STATES_NAMED_IN_ONE_LINE)
        .map(GeneratorState::label)
        .collect();
    let more = states
        .len()
        .saturating_sub(MOST_GENERATOR_STATES_NAMED_IN_ONE_LINE);
    if more == 0 {
        named.join("; ")
    } else {
        format!("{}; and {more} more", named.join("; "))
    }
}
