use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{LazyLock, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rc_game::enemies::shadow_matrix;
use rc_game::npcs::{
    MirroredNpc, NPC_MIRROR_DELAY_MS, NPC_SEND_INTERVAL_MS, NpcHoldBlocker, NpcMirror, NpcPlanner,
    NpcRoster, NpcSighting, npc_state, read_npc_hold_blocker, scan_npcs, split_npc_snapshot,
};
use rc_game::offsets::{
    ACTOR_DRAWN_FLAG_OFFSET, ACTOR_SPAWN_SERIAL_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES,
    ACTOR_TRANSFORM_BLOCK_OFFSET, BODY_MOTION_LAYER,
};
use rc_game::puppet::{
    MotionStep, actor_is_live, follow_step, frame_to_seek, read_puppet_layer, seek_layer_frame,
};
use rc_game::{GameMemory, PhaseEntry, Replicant};
use rc_proto::{Message, MotionState, NpcId, NpcState};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetThread, RemotePlayer};
use crate::npcs::game_calls::NpcCalls;
use crate::npcs::hold_hook::{is_installed, set_held};

static SYNC: LazyLock<Mutex<NpcSync>> = LazyLock::new(|| Mutex::new(NpcSync::default()));

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NpcRole {
    Host,
    Guest,
}

#[derive(Default)]
struct NpcSync {
    role: Option<NpcRole>,
    roster: NpcRoster,
    mirror: NpcMirror,
    planner: NpcPlanner,
    epoch: Option<Instant>,
    last_host_send: Option<Instant>,
    tick: u32,
    hidden_by_us: HashMap<u64, i32>,
    last_started: HashMap<NpcId, i32>,
    blocker: Option<NpcHoldBlocker>,
    last_report: Option<(usize, usize, usize)>,
    idle_reason: Option<&'static str>,
}

struct LocalView<'a> {
    game: &'a Replicant<InProcessMemory>,
    build: &'a VerifiedBuild,
    calls: NpcCalls,
    phase: u16,
}

impl NpcSync {
    fn go_idle(&mut self, build: &VerifiedBuild, reason: &'static str) {
        self.release_all(build);
        self.role = None;
        self.roster.reset();
        self.mirror.clear();
        self.planner.reset();
        self.last_host_send = None;
        self.last_report = None;
        if self.idle_reason != Some(reason) {
            self.idle_reason = Some(reason);
            log_line(&format!("npcs: idle, {reason}"));
        }
    }

    fn take_role(&mut self, build: &VerifiedBuild, role: NpcRole) {
        self.idle_reason = None;
        if self.role != Some(role) {
            self.release_all(build);
            self.roster.reset();
            self.mirror.clear();
            self.planner.reset();
            self.last_host_send = None;
            self.last_report = None;
            self.role = Some(role);
            log_line(&format!(
                "npcs: {} (hold hook {})",
                match role {
                    NpcRole::Host => "host, sending this game's NPCs",
                    NpcRole::Guest => "guest, following the host's NPCs",
                },
                if is_installed() { "on" } else { "off" }
            ));
        }
    }

    fn now_ms(&mut self) -> u64 {
        let epoch = *self.epoch.get_or_insert_with(Instant::now);
        u64::try_from(epoch.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    fn release_all(&mut self, build: &VerifiedBuild) {
        set_held(HashSet::new());
        self.last_started.clear();
        let mut memory = InProcessMemory::new(build);
        for (actor, serial) in self.hidden_by_us.drain() {
            show_again(&mut memory, actor, serial);
        }
    }
}

pub fn tick_npc_sync(build: &VerifiedBuild) {
    let mut sync = SYNC.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(net) = net::handle() else {
        sync.go_idle(build, "co-op is off");
        return;
    };
    let relayed = net.take_relayed(|message| matches!(message, Message::NpcSnapshot(_)));
    let view = net.view();
    let Some(remote) = view.remote else {
        sync.go_idle(build, "no other player is connected");
        return;
    };
    let role = if remote.slot == 1 {
        NpcRole::Host
    } else {
        NpcRole::Guest
    };
    sync.take_role(build, role);
    let game = Replicant::new(InProcessMemory::new(build));
    let Some(phase) = local_phase(&game) else {
        sync.go_idle(build, "own phase not readable (loading?)");
        return;
    };
    let local = LocalView {
        game: &game,
        build,
        calls: NpcCalls::new(build),
        phase,
    };
    match role {
        NpcRole::Host => host_step(&mut sync, net, &local, &remote),
        NpcRole::Guest => guest_step(&mut sync, &local, relayed),
    }
}

fn local_phase(game: &Replicant<InProcessMemory>) -> Option<u16> {
    let name = game.current_phase().ok()?;
    u16::try_from(PhaseEntry::index_of(&name)?).ok()
}

fn host_step(sync: &mut NpcSync, net: &NetThread, local: &LocalView<'_>, remote: &RemotePlayer) {
    let now = Instant::now();
    if sync.last_host_send.is_some_and(|sent| {
        now.saturating_duration_since(sent) < Duration::from_millis(NPC_SEND_INTERVAL_MS)
    }) {
        return;
    }
    sync.last_host_send = Some(now);
    let sightings = scan_npcs(local.game.memory()).unwrap_or_default();
    let update = sync.roster.observe(local.phase, &sightings);
    if update.phase_changed || !update.arrived.is_empty() || !update.gone.is_empty() {
        log_line(&format!(
            "npcs: host phase {} has {} NPCs ({} new, {} gone)",
            local.phase,
            update.seen.len(),
            update.arrived.len(),
            update.gone.len()
        ));
    }
    if remote.state.phase != local.phase {
        return;
    }
    let states: Vec<NpcState> = update
        .seen
        .iter()
        .filter_map(|(id, sighting)| npc_state(*id, sighting))
        .collect();
    sync.tick = sync.tick.wrapping_add(1);
    let players = [
        local
            .game
            .position()
            .map_or(remote.state.position, |at| [at.x, at.y, at.z]),
        remote.state.position,
    ];
    for part in split_npc_snapshot(sync.tick, local.phase, states, &players) {
        net.send_relayed(Message::NpcSnapshot(part));
    }
}

fn guest_step(sync: &mut NpcSync, local: &LocalView<'_>, relayed: Vec<Message>) {
    let now_ms = sync.now_ms();
    for message in relayed {
        if let Message::NpcSnapshot(snapshot) = message {
            sync.mirror.receive_snapshot(now_ms, &snapshot);
        }
    }
    sync.mirror.prune(now_ms);
    let sightings = scan_npcs(local.game.memory()).unwrap_or_default();
    let update = sync.roster.observe(local.phase, &sightings);
    if update.phase_changed {
        sync.planner.reset();
        sync.last_started.clear();
    }
    let blocker = read_npc_hold_blocker(local.game.memory()).ok().flatten();
    if blocker != sync.blocker {
        log_line(&format!(
            "npcs: {}",
            match blocker {
                Some(NpcHoldBlocker::EventScene) => "event scene, NPCs run on their own",
                Some(NpcHoldBlocker::Talk) => "conversation, NPCs run on their own",
                None => "following the host's NPCs again",
            }
        ));
        sync.blocker = blocker;
    }
    if blocker.is_some() || sync.mirror.phase() != Some(local.phase) {
        sync.release_all(local.build);
        return;
    }
    let local_by_id: BTreeMap<NpcId, NpcSighting> = update.seen.iter().copied().collect();
    let local_ids: Vec<NpcId> = local_by_id.keys().copied().collect();
    let samples = sync.mirror.sample(now_ms, NPC_MIRROR_DELAY_MS);
    let plan = sync
        .planner
        .plan(now_ms, &local_ids, &samples, sync.mirror.is_current(now_ms));
    let mut memory = InProcessMemory::new(local.build);
    let mut held = HashSet::new();
    let mut hidden_now = HashSet::new();
    let mut driven = 0_usize;
    for host in &plan.drive {
        let Some(sighting) = local_by_id.get(&host.id) else {
            continue;
        };
        held.insert(sighting.actor);
        if !host.visible {
            hide(sync, &mut memory, sighting);
            hidden_now.insert(sighting.actor);
            continue;
        }
        if drive(sync, local, &mut memory, sighting, host) {
            driven += 1;
        }
    }
    for id in &plan.hide {
        if let Some(sighting) = local_by_id.get(id) {
            held.insert(sighting.actor);
            hide(sync, &mut memory, sighting);
            hidden_now.insert(sighting.actor);
        }
    }
    let shown: Vec<(u64, i32)> = sync
        .hidden_by_us
        .iter()
        .filter(|(actor, _)| !hidden_now.contains(*actor))
        .map(|(actor, serial)| (*actor, *serial))
        .collect();
    for (actor, serial) in shown {
        sync.hidden_by_us.remove(&actor);
        show_again(&mut memory, actor, serial);
    }
    set_held(held);
    for id in &plan.newly_hidden {
        log_line(&format!(
            "npcs: hid {} (the host has no such NPC)",
            id_text(*id)
        ));
    }
    for id in &plan.newly_missing {
        log_line(&format!(
            "npcs: host NPC {} has no match here, not shown",
            id_text(*id)
        ));
    }
    let report = (driven, plan.hide.len(), plan.missing_here.len());
    if sync.last_report != Some(report) {
        sync.last_report = Some(report);
        log_line(&format!(
            "npcs: phase {} local {} host {} driven {} hidden {} waiting {} missing here {}",
            local.phase,
            local_ids.len(),
            sync.mirror.host_total().unwrap_or(0),
            driven,
            plan.hide.len(),
            plan.waiting.len(),
            plan.missing_here.len()
        ));
    }
}

fn drive(
    sync: &mut NpcSync,
    local: &LocalView<'_>,
    memory: &mut InProcessMemory,
    sighting: &NpcSighting,
    host: &MirroredNpc,
) -> bool {
    let Ok(block) = memory
        .read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(sighting.actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
    else {
        return false;
    };
    let matrix = shadow_matrix(&block, host.position, host.yaw_radians);
    if !local.calls.set_transform(memory, sighting.actor, &matrix) {
        return false;
    }
    if let Some(motion) = host.motion {
        follow_motion(sync, local, memory, sighting.actor, host.id, motion);
    }
    true
}

fn follow_motion(
    sync: &mut NpcSync,
    local: &LocalView<'_>,
    memory: &mut InProcessMemory,
    actor: u64,
    id: NpcId,
    target: MotionState,
) {
    let index = i32::from(target.index);
    let layer = read_puppet_layer(memory, actor, BODY_MOTION_LAYER)
        .ok()
        .flatten();
    let last_started = sync.last_started.get(&id).copied();
    match follow_step(layer.map(|layer| layer.motion), last_started, index) {
        MotionStep::Start { blend } => {
            local.calls.play_body_motion(
                actor,
                index,
                blend,
                target.looping,
                target.speed,
                target.frame,
            );
            sync.last_started.insert(id, index);
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
                let _ = seek_layer_frame(memory, &layer, frame);
            }
        }
    }
}

fn hide(sync: &mut NpcSync, memory: &mut InProcessMemory, sighting: &NpcSighting) {
    if sync.hidden_by_us.contains_key(&sighting.actor) || !sighting.visible {
        return;
    }
    if memory
        .write_u8(sighting.actor + ACTOR_DRAWN_FLAG_OFFSET, 0)
        .is_ok()
    {
        sync.hidden_by_us
            .insert(sighting.actor, sighting.spawn_serial);
    }
}

fn show_again(memory: &mut InProcessMemory, actor: u64, serial: i32) {
    // Only an actor that is still the one we hid gets its flag back; a freed slot is left alone.
    let still_ours = actor_is_live(memory, actor).unwrap_or(false)
        && memory
            .read_i32(actor + ACTOR_SPAWN_SERIAL_OFFSET)
            .is_ok_and(|now| now == serial);
    if still_ours {
        let _ = memory.write_u8(actor + ACTOR_DRAWN_FLAG_OFFSET, 1);
    }
}

fn id_text(id: NpcId) -> String {
    format!("{}:{}:{}", id.group, id.class, id.ordinal)
}
