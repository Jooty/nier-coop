use std::collections::{BTreeMap, VecDeque};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use rc_game::enemies::{
    ENEMY_MIRROR_DELAY_MS, EnemyMirror, EnemyRoster, EnemySighting, MirroredEnemy, RosterUpdate,
    enemy_state, match_enemies, scan_enemies, shadow_matrix, split_snapshot,
};
use rc_game::offsets::{ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET};
use rc_game::{GameMemory, PhaseEntry, Replicant};
use rc_proto::{EnemyGone, EnemyGoneReason, EnemyId, EnemySpawned, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::MAX_RECENT_ENEMY_EVENTS;
use crate::enemies::game_calls::EnemyCalls;
use crate::enemies::status::{
    EnemySyncRole, EnemySyncStatus, enemy_id_text, publish_status, snap_enabled,
};
use crate::memory::InProcessMemory;
use crate::net::{self, NetThread, RemotePlayer, SEND_INTERVAL};

static SYNC: LazyLock<Mutex<EnemySync>> = LazyLock::new(|| Mutex::new(EnemySync::default()));

#[derive(Default)]
struct EnemySync {
    role: Option<EnemySyncRole>,
    roster: EnemyRoster,
    mirror: EnemyMirror,
    epoch: Option<Instant>,
    last_host_send: Option<Instant>,
    tick: u32,
    events: VecDeque<String>,
}

struct LocalView<'a> {
    game: &'a Replicant<InProcessMemory>,
    calls: EnemyCalls,
    phase: u16,
}

impl EnemySync {
    fn go_idle(&mut self, reason: &'static str) {
        self.role = None;
        self.roster.reset();
        self.mirror.clear();
        self.last_host_send = None;
        let mut status = EnemySyncStatus::idle(reason);
        status.recent_events.clone_from(&self.events);
        publish_status(status);
    }

    fn take_role(&mut self, role: EnemySyncRole) {
        if self.role != Some(role) {
            self.roster.reset();
            self.mirror.clear();
            self.last_host_send = None;
            self.role = Some(role);
        }
    }

    fn now_ms(&mut self) -> u64 {
        let epoch = *self.epoch.get_or_insert_with(Instant::now);
        u64::try_from(epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn note(&mut self, event: String) {
        log_line(&format!("enemies: {event}"));
        if self.events.len() >= MAX_RECENT_ENEMY_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    fn status(&self) -> EnemySyncStatus {
        EnemySyncStatus {
            idle_reason: None,
            role: self.role,
            recent_events: self.events.clone(),
            ..EnemySyncStatus::default()
        }
    }
}

pub fn tick_enemy_sync(build: &VerifiedBuild) {
    let mut sync = SYNC.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(net) = net::handle() else {
        sync.go_idle("co-op is off");
        return;
    };
    let relayed = net.take_relayed(|message| {
        matches!(
            message,
            Message::EnemySnapshot(_) | Message::EnemySpawned(_) | Message::EnemyGone(_)
        )
    });
    let view = net.view();
    let Some(remote) = view.remote else {
        sync.go_idle("no other player is connected");
        return;
    };
    let role = if remote.slot == 1 {
        EnemySyncRole::Host
    } else {
        EnemySyncRole::Guest
    };
    sync.take_role(role);
    let game = Replicant::new(InProcessMemory::new(build));
    let Some(phase) = local_phase(&game) else {
        sync.go_idle("your own phase is not readable (loading?)");
        return;
    };
    let local = LocalView {
        game: &game,
        calls: EnemyCalls::new(build),
        phase,
    };
    match role {
        EnemySyncRole::Host => host_step(&mut sync, net, &local, &remote),
        EnemySyncRole::Guest => guest_step(&mut sync, &local, relayed),
    }
}

fn local_phase(game: &Replicant<InProcessMemory>) -> Option<u16> {
    let name = game.current_phase().ok()?;
    u16::try_from(PhaseEntry::index_of(&name)?).ok()
}

fn scan(local: &LocalView<'_>) -> Vec<EnemySighting> {
    let memory = local.game.memory();
    let calls = local.calls;
    scan_enemies(
        memory,
        local.game.playable_actor().ok(),
        |group, index| calls.actor_in_group(group, index),
        |actor| calls.hp_of(memory, actor),
    )
    .unwrap_or_default()
}

fn host_step(sync: &mut EnemySync, net: &NetThread, local: &LocalView<'_>, remote: &RemotePlayer) {
    let now = Instant::now();
    if sync
        .last_host_send
        .is_some_and(|sent| now.saturating_duration_since(sent) < SEND_INTERVAL)
    {
        return;
    }
    sync.last_host_send = Some(now);
    let sightings = scan(local);
    let update = sync.roster.observe(local.phase, &sightings);
    note_roster_changes(sync, &update);
    let mut status = sync.status();
    status.local_count = update.seen.len();
    status.host_total = Some(u16::try_from(update.seen.len()).unwrap_or(u16::MAX));
    if remote.state.phase != local.phase {
        status.idle_reason = Some("the other player is in another phase, nothing is sent");
        publish_status(status);
        return;
    }
    let states: BTreeMap<EnemyId, _> = update
        .seen
        .iter()
        .filter_map(|(id, sighting)| Some((*id, enemy_state(*id, sighting)?)))
        .collect();
    for id in &update.spawned {
        if let Some(enemy) = states.get(id) {
            net.send_relayed(Message::EnemySpawned(EnemySpawned {
                phase: local.phase,
                enemy: *enemy,
            }));
        }
    }
    for (id, reason) in &update.gone {
        net.send_relayed(Message::EnemyGone(EnemyGone {
            phase: local.phase,
            id: *id,
            reason: *reason,
        }));
    }
    sync.tick = sync.tick.wrapping_add(1);
    let players = [
        local
            .game
            .position()
            .map_or(remote.state.position, |at| [at.x, at.y, at.z]),
        remote.state.position,
    ];
    let parts = split_snapshot(
        sync.tick,
        local.phase,
        states.into_values().collect(),
        &players,
    );
    status.sent_parts = parts.len();
    for part in parts {
        net.send_relayed(Message::EnemySnapshot(part));
    }
    publish_status(status);
}

fn guest_step(sync: &mut EnemySync, local: &LocalView<'_>, relayed: Vec<Message>) {
    let now_ms = sync.now_ms();
    for message in relayed {
        receive(sync, now_ms, &message);
    }
    sync.mirror.prune(now_ms);
    let sightings = scan(local);
    let update = sync.roster.observe(local.phase, &sightings);
    let mut status = sync.status();
    status.local_count = update.seen.len();
    status.host_total = sync.mirror.host_total();
    status.mirrored = sync.mirror.len();
    if sync.mirror.phase() != Some(local.phase) {
        status.idle_reason = Some("no enemy snapshot from the host for your phase yet");
        publish_status(status);
        return;
    }
    let samples: BTreeMap<EnemyId, MirroredEnemy> = sync
        .mirror
        .sample(now_ms, ENEMY_MIRROR_DELAY_MS)
        .into_iter()
        .map(|enemy| (enemy.id, enemy))
        .collect();
    let here: Vec<(EnemyId, [f32; 3])> = update
        .seen
        .iter()
        .map(|(id, sighting)| (*id, sighting.position))
        .collect();
    let there: Vec<(EnemyId, [f32; 3])> = samples
        .values()
        .map(|enemy| (enemy.id, enemy.position))
        .collect();
    let report = match_enemies(&here, &there);
    status.matched = report.matched.len();
    status.worst_error = report.worst_error();
    status.unmatched_local = report.unmatched_local;
    status.unmatched_remote = report.unmatched_remote;
    if snap_enabled() {
        for (id, sighting) in &update.seen {
            let Some(host) = samples.get(id) else {
                continue;
            };
            if snap(local, sighting.actor, host) {
                status.snapped += 1;
            } else {
                status.snap_failures += 1;
            }
        }
    }
    publish_status(status);
}

fn receive(sync: &mut EnemySync, now_ms: u64, message: &Message) {
    match message {
        Message::EnemySnapshot(snapshot) => sync.mirror.receive_snapshot(now_ms, snapshot),
        Message::EnemySpawned(spawned) => {
            sync.mirror.receive_spawned(now_ms, spawned);
            sync.note(format!("host spawned {}", enemy_id_text(spawned.enemy.id)));
        }
        Message::EnemyGone(gone) => {
            sync.mirror.receive_gone(gone);
            sync.note(format!(
                "host {} {}",
                gone_text(gone.reason),
                enemy_id_text(gone.id)
            ));
        }
        _ => {}
    }
}

fn snap(local: &LocalView<'_>, actor: u64, host: &MirroredEnemy) -> bool {
    let memory = local.game.memory();
    let Ok(block) =
        memory.read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
    else {
        return false;
    };
    let matrix = shadow_matrix(&block, host.position, host.yaw_radians);
    if !local.calls.set_transform(memory, actor, &matrix) {
        return false;
    }
    let Ok(host_hp) = i32::try_from(host.hp) else {
        return true;
    };
    // A host HP of zero means it is dying there; its death arrives as an event instead.
    if host_hp > 0
        && let Some((hp, _)) = local.calls.hp_of(memory, actor)
        && hp != host_hp
    {
        return local.calls.set_hp(memory, actor, host_hp);
    }
    true
}

fn note_roster_changes(sync: &mut EnemySync, update: &RosterUpdate) {
    for id in &update.spawned {
        sync.note(format!("spawned {}", enemy_id_text(*id)));
    }
    for (id, reason) in &update.gone {
        sync.note(format!("{} {}", gone_text(*reason), enemy_id_text(*id)));
    }
}

fn gone_text(reason: EnemyGoneReason) -> &'static str {
    match reason {
        EnemyGoneReason::Killed => "killed",
        EnemyGoneReason::Removed => "removed",
    }
}
