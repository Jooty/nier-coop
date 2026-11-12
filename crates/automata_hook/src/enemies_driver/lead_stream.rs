use std::collections::HashMap;
use std::time::{Duration, Instant};

use automata_game::enemies::{EnemyId, EnemySighting};
use automata_game::enemy_lead::{
    EnemyLead, EnemyPose, HOLD_LIFTS_AFTER_THE_HOSTS_SILENCE_MS, LeadTarget, PlaceStep,
    PlayerPlaces, facing_to_write, guest_target_handle, lead_parts, lead_target_of, place_line,
    place_step, target_wanted_on_the_host,
};
use automata_game::save_redirect::SaveRole;
use automata_proto::Message;
use nc_hook::diagnostics::log_line;
use nc_hook::net::NetThread;

use crate::enemies_driver::driver_state::EnemiesDriver;
use crate::enemies_driver::lead_state::LedEnemy;
use crate::game_calls::EnemyReading;
use crate::game_hooks::{
    LedBody, hooked_starts_of, partner_body_handle, set_led_bodies, take_hold_counts,
    while_a_lead_start_runs,
};
use crate::net::{AutomataNet, local_in_play};

impl EnemiesDriver {
    /// The census step of the host-led enemies: the host reads and sends its shared enemies,
    /// the guest sets up which of them its hooks hold. Both log places every two seconds.
    pub(super) fn lead_enemies(
        &mut self,
        net: &NetThread<AutomataNet>,
        now_ms: u64,
        mine: &[EnemyReading],
    ) {
        if !self.lead.active {
            return;
        }
        let shared = self.enemies_both_copies_hold(mine);
        let paused = !local_in_play() || self.calls.read_game_is_paused();
        let log_round = self.lead.place_log.round_is_due(now_ms);
        match self.role {
            SaveRole::Host => self.lead_as_the_host(net, &shared, paused, log_round),
            SaveRole::Guest => self.follow_as_the_guest(now_ms, &shared, paused, log_round),
        }
        if log_round {
            let counts = take_hold_counts();
            if counts.own_starts_held > 0 || counts.targets_replaced > 0 {
                log_line(&format!(
                    "enemies: in the last two seconds {} own state starts were held back and {} player targets replaced",
                    counts.own_starts_held, counts.targets_replaced
                ));
            }
        }
    }

    /// Every frame on the guest: each held enemy plays the host's new state starts and is eased
    /// to the host's place and facing.
    pub(super) fn follow_the_lead(&mut self, now_ms: u64) {
        if !self.lead.active
            || self.role != SaveRole::Guest
            || self.lead.enemies.is_empty()
            || !local_in_play()
            || self.calls.read_game_is_paused()
        {
            return;
        }
        for enemy in self.lead.enemies.clone() {
            if self.calls.entity_handle_of_body(enemy.body) != Some(enemy.handle)
                || self
                    .calls
                    .read_enemy_health(enemy.body)
                    .is_none_or(|health| health < 1)
                || !self.lead.follower.holds(enemy.id, now_ms)
            {
                continue;
            }
            let Some(pose) = self.lead.follower.fresh_pose(enemy.id, now_ms) else {
                continue;
            };
            self.play_the_hosts_start(enemy, now_ms);
            self.ease_to_the_hosts_place(enemy, &pose);
        }
    }

    pub(super) fn take_leads(&mut self, net: &NetThread<AutomataNet>) {
        let leads = net.update(|shared| core::mem::take(&mut shared.extras.enemy_leads));
        if self.role != SaveRole::Guest || !self.lead.active {
            return;
        }
        for lead in leads {
            self.hear_lead(&lead);
        }
    }

    fn hear_lead(&mut self, lead: &EnemyLead) {
        let now_ms = self.started.elapsed().as_millis() as u64;
        self.lead.follower.hear(now_ms, lead);
        if !self.lead.first_pose_heard && !lead.poses.is_empty() {
            self.lead.first_pose_heard = true;
            log_line(&format!(
                "enemies: first lead from the host: {} enemies stand, face and act as the host's do from now (a guess until it has run)",
                lead.poses.len()
            ));
        }
    }

    pub(super) fn stop_leading(&mut self) {
        self.lead.tally.forget();
        self.lead.follower.forget();
        self.lead.enemies.clear();
        set_led_bodies(HashMap::new());
    }

    fn enemies_both_copies_hold(&self, mine: &[EnemyReading]) -> Vec<EnemyReading> {
        let theirs: HashMap<EnemyId, &EnemySighting> = self
            .other_roster
            .iter()
            .flatten()
            .map(|sighting| (sighting.id, sighting))
            .collect();
        mine.iter()
            .filter(|reading| {
                theirs
                    .get(&reading.sighting.id)
                    .is_some_and(|other| other.object_id == reading.sighting.object_id)
            })
            .copied()
            .collect()
    }

    fn lead_as_the_host(
        &mut self,
        net: &NetThread<AutomataNet>,
        shared: &[EnemyReading],
        paused: bool,
        log_round: bool,
    ) {
        let hero = self.calls.current_hero();
        let hero_handle = (hero != 0)
            .then(|| self.calls.entity_handle_of_body(hero))
            .flatten();
        let partner_handle = partner_body_handle();
        let players = PlayerPlaces {
            host_player: (hero != 0)
                .then(|| self.calls.body_position(hero))
                .flatten()
                .map(|place| [place[0], place[1], place[2]]),
            guest_player: partner_handle
                .and_then(|handle| self.calls.body_of_handle(handle))
                .and_then(|body| self.calls.body_position(body))
                .map(|place| [place[0], place[1], place[2]]),
        };
        let mut poses = Vec::new();
        let mut bodies = HashMap::new();
        for reading in shared {
            let Some(read) = self.calls.read_enemy_pose(reading.body) else {
                continue;
            };
            let id = reading.sighting.id;
            let (starts, state) =
                self.lead
                    .tally
                    .tally(id, read.state, hooked_starts_of(reading.body));
            let mut target = lead_target_of(read.target, hero_handle, partner_handle);
            let mut wanted_handle = None;
            if self.lead.targets_are_hooked
                && let Some(wanted) = target_wanted_on_the_host(read.position, players, target)
            {
                wanted_handle = match wanted {
                    LeadTarget::HostPlayer => hero_handle,
                    LeadTarget::GuestPlayer => partner_handle,
                    LeadTarget::Nobody | LeadTarget::SomethingElse => None,
                };
                if let Some(handle) = wanted_handle
                    && handle != read.target
                {
                    self.write_target(reading.body, id, handle, wanted);
                }
                target = wanted;
            }
            bodies.insert(
                reading.body,
                LedBody {
                    handle: reading.handle,
                    held_until: None,
                    target: wanted_handle,
                    player_handles: [hero_handle.unwrap_or(0), partner_handle.unwrap_or(0)],
                },
            );
            poses.push(EnemyPose {
                id,
                position: read.position,
                facing: read.facing,
                state,
                starts,
                target,
            });
            if log_round && self.lead.place_log.take_line() {
                log_line(&place_line(
                    id,
                    read.position,
                    read.state,
                    "led by this copy (host)",
                ));
            }
        }
        set_led_bodies(bodies);
        let ids: Vec<EnemyId> = poses.iter().map(|pose| pose.id).collect();
        self.lead.tally.keep_only(&ids);
        // A paused host sends nothing, so the guest's hold lifts within a second.
        if !paused {
            for part in lead_parts(&poses) {
                net.send_relayed(Message::EnemyLead(part));
            }
        }
    }

    fn follow_as_the_guest(
        &mut self,
        now_ms: u64,
        shared: &[EnemyReading],
        paused: bool,
        log_round: bool,
    ) {
        let hero = self.calls.current_hero();
        let hero_handle = (hero != 0)
            .then(|| self.calls.entity_handle_of_body(hero))
            .flatten();
        let partner_handle = partner_body_handle();
        let mut bodies = HashMap::new();
        let mut enemies = Vec::new();
        for reading in shared {
            let id = reading.sighting.id;
            self.lead.follower.note_body(id, reading.handle, now_ms);
            let held = !paused && !self.defeats.holds(id) && self.lead.follower.holds(id, now_ms);
            let pose = self.lead.follower.fresh_pose(id, now_ms);
            let target =
                pose.and_then(|pose| guest_target_handle(pose.target, hero_handle, partner_handle));
            if held {
                let silent_ms = self
                    .lead
                    .follower
                    .heard_at_ms(id)
                    .map_or(HOLD_LIFTS_AFTER_THE_HOSTS_SILENCE_MS, |at_ms| {
                        now_ms.saturating_sub(at_ms)
                    });
                let left_ms = HOLD_LIFTS_AFTER_THE_HOSTS_SILENCE_MS.saturating_sub(silent_ms);
                bodies.insert(
                    reading.body,
                    LedBody {
                        handle: reading.handle,
                        held_until: Some(Instant::now() + Duration::from_millis(left_ms)),
                        target,
                        player_handles: [hero_handle.unwrap_or(0), partner_handle.unwrap_or(0)],
                    },
                );
                enemies.push(LedEnemy {
                    id,
                    body: reading.body,
                    handle: reading.handle,
                });
            }
            let read = self.calls.read_enemy_pose(reading.body);
            if held
                && self.lead.targets_are_hooked
                && let (Some(read), Some(target), Some(pose)) = (read, target, pose)
                && read.target != target
                && [hero_handle, partner_handle].contains(&Some(read.target))
            {
                self.write_target(reading.body, id, target, pose.target);
            }
            if log_round
                && let Some(read) = read
                && self.lead.place_log.take_line()
            {
                log_line(&place_line(
                    id,
                    read.position,
                    read.state,
                    guest_place_note(held, pose.as_ref()),
                ));
            }
        }
        set_led_bodies(bodies);
        let ids: Vec<EnemyId> = shared.iter().map(|reading| reading.sighting.id).collect();
        self.lead.follower.keep_only(&ids, now_ms);
        self.lead.enemies = enemies;
    }

    #[expect(
        unsafe_code,
        reason = "starts the host's state on an enemy resolved from its live handle this frame"
    )]
    fn play_the_hosts_start(&mut self, enemy: LedEnemy, now_ms: u64) {
        if !self.lead.states_are_hooked {
            return;
        }
        let Some(state) = self.lead.follower.start_to_play(enemy.id, now_ms) else {
            return;
        };
        // SAFETY: the body was resolved from its live handle this frame on the game thread and
        // is a shared enemy with health left.
        let started =
            while_a_lead_start_runs(|| unsafe { self.calls.start_enemy_state(enemy.body, state) });
        if !self.lead.first_start_played {
            self.lead.first_start_played = true;
            log_line(&format!(
                "enemies: first state start from the host played on {}: state {:#X} {} {} {} (started: {started})",
                enemy.id.label(),
                state[0],
                state[1],
                state[2],
                state[3]
            ));
        }
    }

    fn ease_to_the_hosts_place(&mut self, enemy: LedEnemy, pose: &EnemyPose) {
        let Some(read) = self.calls.read_enemy_pose(enemy.body) else {
            return;
        };
        if let PlaceStep::MoveTo(place) = place_step(read.position, pose.position)
            && self.calls.write_enemy_place(enemy.body, place)
            && !self.lead.first_place_written
        {
            self.lead.first_place_written = true;
            log_line(&format!(
                "enemies: first place write toward the host's place on {} (the game may undo plain writes: a guess)",
                enemy.id.label()
            ));
        }
        if let Some(facing) = facing_to_write(read.facing, pose.facing) {
            self.calls.write_enemy_facing(enemy.body, facing);
        }
    }

    fn write_target(&mut self, body: usize, id: EnemyId, handle: u32, target: LeadTarget) {
        if self.calls.write_enemy_target(body, handle) && !self.lead.first_target_written {
            self.lead.first_target_written = true;
            log_line(&format!(
                "enemies: first target write: {} aims at {target:?} (handle {handle:#X})",
                id.label()
            ));
        }
    }
}

fn guest_place_note(held: bool, pose: Option<&EnemyPose>) -> &'static str {
    match (held, pose) {
        (true, _) => "following the host",
        (false, Some(_)) => "not held (new here, defeated or paused), on its own",
        (false, None) => "no fresh pose from the host, on its own",
    }
}
