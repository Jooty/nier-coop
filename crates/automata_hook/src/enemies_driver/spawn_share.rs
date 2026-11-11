use automata_game::enemies::{
    EnemyId, EnemySpawn, GENERATORS_ARE_READ_EVERY_MS, GeneratorState, generators_with_a_shared_id,
};
use automata_game::save_redirect::SaveRole;
use automata_proto::Message;
use nc_hook::diagnostics::log_line;
use nc_hook::net::NetThread;

use crate::enemies_driver::MOST_GENERATOR_STATES_NAMED_IN_ONE_LINE;
use crate::enemies_driver::driver::log_all;
use crate::enemies_driver::driver_state::EnemiesDriver;
use crate::game_calls::EnemyReading;
use crate::game_hooks::{
    set_generators_left_to_the_game, set_guest_generators_held, take_observed_spawns,
    with_guest_generators,
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
        self.read_generators(now_ms);
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
                self.read_the_enemies_made_from_the_hosts_spawns()
            }
        }
    }

    pub(super) fn note_link_change_for_spawns(&mut self, connected: bool) {
        match self.role {
            SaveRole::Host => {
                self.host_spawns.forget_what_the_guest_was_told();
                self.living_spawns_are_told_again = connected;
            }
            SaveRole::Guest => {
                with_guest_generators(|guest| guest.forget());
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

    pub(super) fn hear_gone(&mut self, id: EnemyId) {
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
        if let Some(line) = with_guest_generators(|guest| guest.hear_gone(id, defeated)).flatten() {
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

    fn read_generators(&mut self, now_ms: u64) {
        if self
            .generators_read_at_ms
            .is_some_and(|at_ms| now_ms.saturating_sub(at_ms) < GENERATORS_ARE_READ_EVERY_MS)
        {
            return;
        }
        self.generators_read_at_ms = Some(now_ms);
        let generators: Vec<_> = if local_in_play() {
            self.calls
                .read_generators()
                .into_iter()
                .map(|(_, header)| header)
                .collect()
        } else {
            Vec::new()
        };
        log_all(self.generator_census.note(&generators));
        let left_to_the_game = generators_with_a_shared_id(&generators);
        self.generator_states = generators
            .iter()
            .filter(|generator| !left_to_the_game.contains(&generator.action_id))
            .map(|generator| GeneratorState {
                generator: generator.action_id,
                phase: generator.phase(),
                spawned: u32::try_from(generator.spawned).unwrap_or(u32::MAX),
            })
            .collect();
        set_generators_left_to_the_game(left_to_the_game.clone());
        self.generators_left_to_the_game = left_to_the_game;
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
        reason = "calls the game's entity terminate on an enemy this mod had the generator make"
    )]
    fn read_the_enemies_made_from_the_hosts_spawns(&mut self) -> Vec<EnemyReading> {
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
        }
        let mut readings = Vec::new();
        for (id, handle) in with_guest_generators(|guest| guest.made()).unwrap_or_default() {
            if self.calls.entity_of_handle(handle) == 0 {
                if let Some(line) =
                    with_guest_generators(|guest| guest.note_made_enemy_gone(id)).flatten()
                {
                    self.log_spawn_line(line);
                }
                continue;
            }
            readings.extend(
                self.calls
                    .read_generated_enemy(id, handle)
                    .filter(|reading| reading.reads_as_an_enemy),
            );
        }
        readings
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
