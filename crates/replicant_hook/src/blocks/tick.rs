use std::sync::PoisonError;

use replicant_game::Replicant;
use replicant_game::arrival::partner_shares_phase;
use replicant_game::blocks::{
    BLOCK_READ_BACK_DELAY_MS, BLOCK_SCAN_INTERVAL_MS, BLOCK_STATE_SNAPSHOT_BYTES, HoldAnnouncer,
    HoldChange, KnownBlock, StateTrail, block_glide_start_text, block_is_live, block_known_text,
    block_landed_text, block_missing_text, block_moved_text, block_read_back_text, block_sent_text,
    both_hold_block_text, held_block_text, other_map_text, partner_holds_block_text,
    partner_let_go_text, partner_unconfirmed_text, read_block_position, read_block_sightings,
    state_diff_text, write_block_position, you_hold_block_text, you_let_go_text,
};
use replicant_game::cube_probe::read_block_lossy;
use replicant_game::stay_together::local_phase_index;
use replicant_proto::{BlockHeld, BlockMoved, Message};

use crate::blocks::grab_gate::publish;
use crate::blocks::hold::HeldBlock;
use crate::blocks::state::{BlockState, STATE};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::gimmick_probe::actor_in_group;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};

pub fn tick_blocks(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        release_partner_state();
        return;
    };
    if !net.view().partner_in_play() {
        release_partner_state();
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
    log_hold_changes(&state.partner_holds.set_local_phase(phase));
    log_hold_changes(&state.partner_holds.expire(now));
    let wanted =
        |message: &Message| matches!(message, Message::BlockMoved(_) | Message::BlockHeld(_));
    for message in net.take_relayed(wanted) {
        match message {
            Message::BlockMoved(moved) => apply(build, &mut memory, state, phase, now, moved),
            Message::BlockHeld(held) => apply_held(state, now, held),
            _ => {}
        }
    }
    glide_blocks(&mut memory, state, now);
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
    share_hold(net, state, phase, now, held.map(|held| held.id));
    publish_gate(state, game.playable_actor().unwrap_or(0));
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

fn release_partner_state() {
    let mut guard = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    log_hold_changes(&guard.partner_holds.clear());
    guard.announcer = HoldAnnouncer::default();
    guard.logged_local_hold = None;
    guard.logged_both_holding.clear();
    drop(guard);
    publish(0, Vec::new());
}

fn log_hold_changes(changes: &[HoldChange]) {
    for change in changes {
        log_line(&match change {
            HoldChange::Took(id) => partner_holds_block_text(*id),
            HoldChange::LetGo(id) => partner_let_go_text(*id),
            HoldChange::Unconfirmed(id) => partner_unconfirmed_text(*id),
        });
    }
}

fn apply_held(state: &mut BlockState, now: u64, held: BlockHeld) {
    let change = state
        .partner_holds
        .partner_reports(now, held.phase, held.id, held.held);
    log_hold_changes(change.as_slice());
}

fn share_hold(
    net: &net::NetThread,
    state: &mut BlockState,
    phase: u16,
    now: u64,
    held_id: Option<u16>,
) {
    if state.logged_local_hold != held_id {
        if let Some(old) = state.logged_local_hold {
            log_line(&you_let_go_text(old));
        }
        if let Some(new) = held_id {
            log_line(&you_hold_block_text(new));
        }
        state.logged_local_hold = held_id;
    }
    for announcement in state.announcer.observe(now, phase, held_id) {
        net.send_relayed(Message::BlockHeld(BlockHeld {
            phase: announcement.phase,
            id: announcement.id,
            held: announcement.held,
        }));
    }
    let both: Vec<u16> = held_id
        .filter(|id| state.partner_holds.holds(*id))
        .into_iter()
        .collect();
    state.logged_both_holding.retain(|id| both.contains(id));
    for id in both {
        if state.logged_both_holding.insert(id) {
            log_line(&both_hold_block_text(id));
        }
    }
}

fn publish_gate(state: &BlockState, player: u64) {
    let actors: Vec<(u16, u64)> = state
        .partner_holds
        .held_ids()
        .into_iter()
        .filter_map(|id| Some((id, state.roster.actor_of(id)?)))
        .collect();
    publish(player, actors);
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
    if state.glides.target_of(moved.id).is_none() {
        log_line(&block_glide_start_text(
            moved.id,
            read_block_position(memory, actor),
            moved.position,
        ));
    }
    state.glides.set_target(now, moved.id, moved.position);
    state.outbox.note_remote_move(now, moved.id);
}

fn glide_blocks(memory: &mut InProcessMemory, state: &mut BlockState, now: u64) {
    let roster = &state.roster;
    let steps = state.glides.advance(now, |id| {
        roster
            .actor_of(id)
            .and_then(|actor| read_block_position(memory, actor))
    });
    for step in steps {
        let Some(actor) = state.roster.actor_of(step.id) else {
            continue;
        };
        if write_block_position(memory, actor, step.to).is_err() {
            log_line(&format!("blocks: could not write block {}", step.id));
            continue;
        }
        let target = state.glides.target_of(step.id).unwrap_or(step.to);
        let now_at = read_block_position(memory, actor);
        if step.landed {
            log_line(&block_landed_text(step.id, step.to, now_at));
            state
                .read_backs
                .push((step.id, step.to, now + BLOCK_READ_BACK_DELAY_MS));
        } else if step.log_due {
            log_line(&block_moved_text(step.id, target, now_at));
        }
    }
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
