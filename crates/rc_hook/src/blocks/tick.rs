use std::sync::PoisonError;

use rc_game::Replicant;
use rc_game::arrival::partner_shares_phase;
use rc_game::blocks::{
    BLOCK_READ_BACK_DELAY_MS, BLOCK_SCAN_INTERVAL_MS, BLOCK_STATE_SNAPSHOT_BYTES, KnownBlock,
    StateTrail, block_is_live, block_known_text, block_missing_text, block_moved_text,
    block_read_back_text, block_sent_text, held_block_text, other_map_text, read_block_position,
    read_block_sightings, state_diff_text, write_block_position,
};
use rc_game::cube_probe::read_block_lossy;
use rc_game::stay_together::local_phase_index;
use rc_proto::{BlockMoved, Message};

use crate::blocks::hold::HeldBlock;
use crate::blocks::state::{BlockState, STATE};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::gimmick_probe::actor_in_group;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};

pub fn tick_blocks(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    if !net.view().partner_in_play() {
        return;
    }
    let mut memory = InProcessMemory::new(build);
    let game = Replicant::new(InProcessMemory::new(build));
    let Some(phase) = local_phase_index(&game) else {
        return;
    };
    let now = net_clock_ms();
    let mut guard = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    let state = &mut *guard;
    if state
        .last_scan_ms
        .is_none_or(|last| now.saturating_sub(last) >= BLOCK_SCAN_INTERVAL_MS)
    {
        scan(build, &memory, state, phase, now);
    }
    for message in net.take_relayed(|message| matches!(message, Message::BlockMoved(_))) {
        if let Message::BlockMoved(moved) = message {
            apply(build, &mut memory, state, phase, now, moved);
        }
    }
    report_read_backs(&memory, state, now);
    let blocks: Vec<(KnownBlock, Option<[f32; 3]>)> = state
        .roster
        .blocks()
        .iter()
        .map(|block| (*block, read_block_position(&memory, block.actor)))
        .collect();
    let player_position = game.position().ok().map(|at| [at.x, at.y, at.z]);
    let held = state.hold.update(
        &memory,
        now,
        game.playable_actor().unwrap_or(0),
        player_position,
        &blocks,
    );
    let BlockState { roster, outbox, .. } = &mut *state;
    let step = outbox.observe(now, held.map(|held| held.id), |id| {
        roster
            .actor_of(id)
            .and_then(|actor| read_block_position(&memory, actor))
    });
    if let Some((id, position)) = step {
        net.send_relayed(Message::BlockMoved(BlockMoved {
            phase,
            id,
            position,
        }));
        log_line(&block_sent_text(id, position));
    }
    follow_state_trail(&memory, state, held, step.is_some());
}

fn scan(
    build: &VerifiedBuild,
    memory: &InProcessMemory,
    state: &mut BlockState,
    phase: u16,
    now: u64,
) {
    state.last_scan_ms = Some(now);
    let sightings =
        read_block_sightings(memory, |group, index| actor_in_group(build, group, index));
    for block in state.roster.observe(phase, &sightings) {
        let position = read_block_position(memory, block.actor).unwrap_or_default();
        log_line(&block_known_text(&block, position));
    }
    let held_id = state.trail.as_ref().map(|(id, _)| *id);
    let live: Vec<u16> = state.roster.blocks().iter().map(|block| block.id).collect();
    state.idle_snapshots.retain(|id, _| live.contains(id));
    for block in state.roster.blocks() {
        if held_id != Some(block.id) {
            state
                .idle_snapshots
                .insert(block.id, snapshot(memory, block.actor));
        }
    }
}

fn apply(
    build: &VerifiedBuild,
    memory: &mut InProcessMemory,
    state: &mut BlockState,
    phase: u16,
    now: u64,
    moved: BlockMoved,
) {
    if !partner_shares_phase(Some(phase), moved.phase) {
        log_line(&other_map_text(moved.id));
        return;
    }
    let mut actor = live_actor(memory, state, moved.id);
    if actor.is_none() {
        scan(build, memory, state, phase, now);
        actor = live_actor(memory, state, moved.id);
    }
    let Some(actor) = actor else {
        log_line(&block_missing_text(moved.id, state.roster.blocks().len()));
        return;
    };
    if write_block_position(memory, actor, moved.position).is_err() {
        log_line(&format!("blocks: could not write block {}", moved.id));
        return;
    }
    state.outbox.note_remote_move(now, moved.id);
    state
        .read_backs
        .push((moved.id, moved.position, now + BLOCK_READ_BACK_DELAY_MS));
    log_line(&block_moved_text(
        moved.id,
        moved.position,
        read_block_position(memory, actor),
    ));
}

fn live_actor(memory: &InProcessMemory, state: &BlockState, id: u16) -> Option<u64> {
    state
        .roster
        .actor_of(id)
        .filter(|actor| block_is_live(memory, *actor))
}

fn report_read_backs(memory: &InProcessMemory, state: &mut BlockState, now: u64) {
    let (due, waiting): (Vec<_>, Vec<_>) = state
        .read_backs
        .drain(..)
        .partition(|(_, _, due_ms)| now >= *due_ms);
    state.read_backs = waiting;
    for (id, wanted, _) in due {
        let at = state
            .roster
            .actor_of(id)
            .and_then(|actor| read_block_position(memory, actor));
        log_line(&block_read_back_text(id, wanted, at));
    }
}

fn snapshot(memory: &InProcessMemory, actor: u64) -> Vec<u8> {
    read_block_lossy(memory, actor, BLOCK_STATE_SNAPSHOT_BYTES)
}

fn follow_state_trail(
    memory: &InProcessMemory,
    state: &mut BlockState,
    held: Option<HeldBlock>,
    moved: bool,
) {
    let tracking = state.outbox.tracking().is_some();
    match (held, state.trail.as_ref().map(|(id, _)| *id)) {
        (Some(held), trail_id) if trail_id != Some(held.id) => start_trail(memory, state, held),
        (Some(_), Some(_)) => {
            if moved {
                advance_trail(memory, state, "moving");
            }
        }
        (None, Some(_)) => {
            advance_trail(memory, state, "released");
            if !tracking {
                advance_trail(memory, state, "settled");
                state.trail = None;
            }
        }
        _ => {}
    }
}

fn start_trail(memory: &InProcessMemory, state: &mut BlockState, held: HeldBlock) {
    let Some(actor) = state.roster.actor_of(held.id) else {
        return;
    };
    let current = snapshot(memory, actor);
    let idle = state
        .idle_snapshots
        .get(&held.id)
        .cloned()
        .unwrap_or_else(|| current.clone());
    log_line(&held_block_text(
        held.id,
        held.chosen_by,
        read_block_position(memory, actor).unwrap_or_default(),
    ));
    state.trail = Some((held.id, StateTrail::new("idle", idle)));
    advance_trail(memory, state, "grabbed");
}

fn advance_trail(memory: &InProcessMemory, state: &mut BlockState, stage: &'static str) {
    let Some((id, trail)) = state.trail.as_mut() else {
        return;
    };
    let Some(actor) = state.roster.actor_of(*id) else {
        return;
    };
    if let Some(diff) = trail.advance(stage, snapshot(memory, actor)) {
        for line in state_diff_text(*id, &diff) {
            log_line(&line);
        }
    }
}
