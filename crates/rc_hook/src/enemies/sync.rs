use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::Instant;

use rc_game::enemies::{
    ENEMY_MIRROR_DELAY_MS, EnemyMirror, EnemyRoster, EnemySighting, MirroredEnemy, RosterUpdate,
    enemy_state, match_enemies, partner_actors, scan_enemies, shadow_matrix, split_snapshot,
};
use rc_game::offsets::{
    ACTOR_TIME_SCALE_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET,
    BODY_MOTION_LAYER,
};
use rc_game::puppet::{
    LayerMotion, MOTION_BLEND_SECONDS, MotionStep, follow_step, frame_to_seek, read_puppet_layer,
    seek_layer_frame,
};
use rc_game::{GameMemory, PhaseEntry, Replicant};
use rc_proto::{
    EnemyGone, EnemyGoneReason, EnemyHit, EnemyId, EnemySpawned, Message, MotionState,
    SessionOptions,
};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::combat::CombatState;
use crate::enemies::combat_view::clear_view;
use crate::enemies::game_calls::EnemyCalls;
use crate::enemies::hold_hook::set_held;
use crate::enemies::status::{
    EnemySyncRole, EnemySyncStatus, enemy_id_text, publish_status, snap_enabled,
};
use crate::enemies::{
    MAX_RECENT_ENEMY_EVENTS, MOTION_FRAME_MOVED, MOTION_RESTART_INTERVAL_MS, MOTION_STALL_MS,
    SECOND_MOTION_LAYER,
};
use crate::enemies::{guest_combat, host_combat};
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
    combat: CombatState,
}

pub(super) struct LocalView<'a> {
    pub(super) build: &'a VerifiedBuild,
    pub(super) game: &'a Replicant<InProcessMemory>,
    pub(super) calls: EnemyCalls,
    pub(super) phase: u16,
}

impl EnemySync {
    fn go_idle(&mut self, build: &VerifiedBuild, reason: &'static str) {
        host_combat::restore_enemy_health(&mut self.combat, build);
        self.role = None;
        self.roster.reset();
        self.mirror.clear();
        self.last_host_send = None;
        self.release_combat();
        let mut status = EnemySyncStatus::idle(reason);
        status.recent_events.clone_from(&self.events);
        status.combat = self.combat.counts.with_hook_counts();
        publish_status(status);
    }

    fn take_role(&mut self, build: &VerifiedBuild, role: EnemySyncRole) {
        if self.role != Some(role) {
            host_combat::restore_enemy_health(&mut self.combat, build);
            self.roster.reset();
            self.mirror.clear();
            self.last_host_send = None;
            self.release_combat();
            self.role = Some(role);
        }
    }

    fn release_combat(&mut self) {
        self.combat.reset();
        clear_view();
        set_held(HashSet::new());
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
            combat: self.combat.counts.with_hook_counts(),
            ..EnemySyncStatus::default()
        }
    }
}

pub fn tick_enemy_sync(build: &VerifiedBuild) {
    let mut sync = SYNC.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(net) = net::handle() else {
        sync.go_idle(build, "co-op is off");
        return;
    };
    let relayed = net.take_relayed(|message| {
        matches!(
            message,
            Message::EnemySnapshot(_)
                | Message::EnemySpawned(_)
                | Message::EnemyGone(_)
                | Message::EnemyHit(_)
                | Message::PlayerHit(_)
                | Message::EnemyKilled(_)
                | Message::EnemyDrop(_)
                | Message::EnemyWordDrop(_)
        )
    });
    let view = net.view();
    let Some(remote) = view.remote else {
        sync.go_idle(build, "no other player is connected");
        return;
    };
    let role = if remote.slot == 1 {
        EnemySyncRole::Host
    } else {
        EnemySyncRole::Guest
    };
    sync.take_role(build, role);
    let game = Replicant::new(InProcessMemory::new(build));
    let Some(phase) = local_phase(&game) else {
        sync.go_idle(build, "your own phase is not readable (loading?)");
        return;
    };
    let local = LocalView {
        build,
        game: &game,
        calls: EnemyCalls::new(build),
        phase,
    };
    match role {
        EnemySyncRole::Host => host_step(&mut sync, net, &local, &remote, &relayed, view.options),
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
        &partner_actors(memory),
        |group, index| calls.actor_in_group(group, index),
        |actor| calls.hp_of(memory, actor),
    )
    .unwrap_or_default()
}

fn host_step(
    sync: &mut EnemySync,
    net: &NetThread,
    local: &LocalView<'_>,
    remote: &RemotePlayer,
    relayed: &[Message],
    options: SessionOptions,
) {
    let now_ms = sync.now_ms();
    let hits: Vec<EnemyHit> = relayed
        .iter()
        .filter_map(|message| match message {
            Message::EnemyHit(hit) => Some(*hit),
            _ => None,
        })
        .collect();
    host_combat::apply_guest_hits(&mut sync.combat, local, now_ms, &hits);
    let now = Instant::now();
    if sync
        .last_host_send
        .is_some_and(|sent| now.saturating_duration_since(sent) < SEND_INTERVAL)
    {
        return;
    }
    sync.last_host_send = Some(now);
    let sightings = scan(local);
    let mut update = sync.roster.observe(local.phase, &sightings);
    host_combat::after_scan(
        &mut sync.combat,
        local,
        net,
        now_ms,
        &mut update,
        remote,
        options,
    );
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
        guest_combat::receive(&mut sync.combat, now_ms, &message);
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
        sync.release_combat();
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
    let authority = snap_enabled();
    guest_combat::step(
        &mut sync.combat,
        local,
        now_ms,
        &update.seen,
        &samples,
        authority,
    );
    if authority {
        for (id, sighting) in &update.seen {
            let Some(host) = samples.get(id) else {
                continue;
            };
            if sync.combat.killed_copies.contains(&sighting.actor) {
                continue;
            }
            if let Some(motion) = host.motion {
                follow_motion(&mut sync.combat, local, sighting.actor, *id, motion, now_ms);
            }
            if snap(local, sighting.actor, host) {
                status.snapped += 1;
            } else {
                status.snap_failures += 1;
            }
        }
    }
    status.combat = sync.combat.counts.with_hook_counts();
    publish_status(status);
}

fn follow_motion(
    combat: &mut CombatState,
    local: &LocalView<'_>,
    actor: u64,
    id: EnemyId,
    target: MotionState,
    now_ms: u64,
) {
    let mut memory = InProcessMemory::new(local.build);
    let index = i32::from(target.index);
    let layer = read_puppet_layer(&memory, actor, BODY_MOTION_LAYER)
        .ok()
        .flatten();
    let started = combat.last_started.get(&id).copied();
    let watch = combat.motion_watch.entry(id).or_default();
    let stalled = match layer {
        None => true,
        Some(layer) if layer.motion.index == index && target.speed != 0.0 => {
            if (layer.motion.frame - watch.frame).abs() > MOTION_FRAME_MOVED {
                watch.frame = layer.motion.frame;
                watch.since_ms = now_ms;
                false
            } else {
                now_ms.saturating_sub(watch.since_ms) >= MOTION_STALL_MS
            }
        }
        Some(_) => false,
    };
    if !stalled {
        watch.reported = false;
    } else if !watch.reported {
        watch.reported = true;
        log_motion_stall(
            &memory,
            actor,
            id,
            index,
            target,
            layer.map(|layer| layer.motion),
        );
    }
    let restart_due = stalled
        && watch
            .restarted_at_ms
            .is_none_or(|at| now_ms.saturating_sub(at) >= MOTION_RESTART_INTERVAL_MS);
    // A readable layer playing something else always restarts the target, even after our start.
    let step = match layer {
        _ if restart_due => MotionStep::Start {
            blend: MOTION_BLEND_SECONDS,
        },
        Some(layer) if layer.motion.index != index => MotionStep::Start {
            blend: MOTION_BLEND_SECONDS,
        },
        _ => follow_step(layer.map(|layer| layer.motion), started, index),
    };
    if restart_due {
        watch.restarted_at_ms = Some(now_ms);
        watch.since_ms = now_ms;
    }
    match step {
        MotionStep::Start { blend } => {
            local.calls.play_body_motion(
                actor,
                index,
                blend,
                target.looping,
                target.speed,
                target.frame,
            );
            combat.last_started.insert(id, index);
        }
        MotionStep::Keep => {
            if let Some(layer) = layer.filter(|layer| layer.motion.index == index)
                && let Some(frame) = frame_to_seek(
                    layer.motion.frame,
                    target.frame,
                    layer.frame_count,
                    layer.motion.looping,
                )
            {
                let _ = seek_layer_frame(&mut memory, &layer, frame);
            }
        }
    }
}

fn log_motion_stall(
    memory: &InProcessMemory,
    actor: u64,
    id: EnemyId,
    index: i32,
    target: MotionState,
    body: Option<LayerMotion>,
) {
    let describe = |motion: Option<LayerMotion>| {
        motion.map_or_else(
            || "unreadable".to_owned(),
            |motion| {
                format!(
                    "motion {} frame {:.1} speed {:.2}",
                    motion.index, motion.frame, motion.speed
                )
            },
        )
    };
    let second = read_puppet_layer(memory, actor, SECOND_MOTION_LAYER)
        .ok()
        .flatten()
        .map(|layer| layer.motion);
    let time_scale = memory
        .read_f32(actor + ACTOR_TIME_SCALE_OFFSET)
        .map_or_else(|_| "unreadable".to_owned(), |scale| format!("{scale:.2}"));
    log_line(&format!(
        "combat: your copy of {} stopped animating (host motion {index} frame {:.1} speed {:.2}; body layer {}; layer 1 {}; time scale {time_scale}); restarting the host's motion",
        enemy_id_text(id),
        target.frame,
        target.speed,
        describe(body),
        describe(second)
    ));
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
