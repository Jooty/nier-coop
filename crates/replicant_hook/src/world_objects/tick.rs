use std::sync::PoisonError;

use replicant_game::Replicant;
use replicant_game::arrival::partner_shares_phase;
use replicant_game::blocks::{HoldChange, read_block_position, write_block_position};
use replicant_game::stay_together::local_phase_index;
use replicant_game::world_objects::{
    BreakVerdict, HOST_RESENDS_MOVED_OBJECTS_EVERY_MS, HoldHow, MoveVerdict, ObjectSync,
    PARTNER_BREAK_RETRY_FOR_MS, REFUSAL_ANSWER_INTERVAL_MS, WORLD_OBJECT_SCAN_INTERVAL_MS,
    cube_is_hittable, handle_owner, key_text, kind_of_vtable_rva, object_is_live,
    object_known_text, partner_break_text, partner_break_verdict, partner_let_go_text,
    partner_move_verdict, partner_moves_text, partner_took_text, read_held_target,
    read_object_sightings, refused_guest_move_text, unlisted_hold_text, you_broke_text,
    you_let_go_text, you_move_text, you_took_text,
};
use replicant_proto::{Message, WorldObjectBroken, WorldObjectKey, WorldObjectMoved};

use crate::blocks::publish_partner_held_objects;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::gimmick_probe::actor_in_group;
use crate::memory::InProcessMemory;
use crate::net::{self, NetSharedExt, net_clock_ms};
use crate::world_objects::break_hook::{break_here, take_local_breaks};
use crate::world_objects::state::{ObjectState, STATE};

struct Tick<'a> {
    build: &'a VerifiedBuild,
    memory: InProcessMemory,
    net: &'static net::NetThread,
    phase: u16,
    now: u64,
    i_am_host: bool,
}

impl Tick<'_> {
    fn scan(&self, state: &mut ObjectState) {
        state.last_scan_ms = Some(self.now);
        let sightings = read_object_sightings(&self.memory, |group, index| {
            actor_in_group(self.build, group, index)
        });
        for object in state.roster.observe(self.phase, &sightings) {
            let position = read_block_position(&self.memory, object.actor).unwrap_or_default();
            log_line(&object_known_text(&object.key, object.actor, position));
        }
    }

    fn position_of(&self, state: &ObjectState, id: u16) -> Option<[f32; 3]> {
        state
            .roster
            .actor_of(id)
            .and_then(|actor| read_block_position(&self.memory, actor))
    }

    fn id_of_key(&self, state: &mut ObjectState, key: &WorldObjectKey) -> Option<u16> {
        if let Some(id) = state.roster.id_of_key(key) {
            return Some(id);
        }
        self.scan(state);
        state.roster.id_of_key(key)
    }

    fn send_moved(&self, key: WorldObjectKey, position: [f32; 3], held: bool) {
        let moved = WorldObjectMoved {
            phase: self.phase,
            key,
            position,
            held,
        };
        if moved.validate().is_ok() {
            self.net.send_relayed(Message::WorldObjectMoved(moved));
        }
    }

    fn log_partner_hold_changes(&self, state: &ObjectState, changes: &[HoldChange]) {
        for change in changes {
            let (HoldChange::LetGo(id) | HoldChange::Unconfirmed(id) | HoldChange::Took(id)) =
                *change;
            let Some(key) = state.roster.key_of(id) else {
                continue;
            };
            let theirs = state
                .partner_last_heard
                .get(&id)
                .copied()
                .unwrap_or_default();
            log_line(&match change {
                HoldChange::Took(_) => partner_took_text(&key, theirs),
                HoldChange::LetGo(_) | HoldChange::Unconfirmed(_) => {
                    partner_let_go_text(&key, theirs)
                }
            });
        }
    }

    fn partner_moved(&self, state: &mut ObjectState, moved: WorldObjectMoved) {
        if !partner_shares_phase(Some(self.phase), moved.phase) {
            return;
        }
        let Some(id) = self.id_of_key(state, &moved.key) else {
            if state.refusal_log.due(self.now, u16::MAX) {
                log_line(&format!(
                    "objects: no {} here ({} known), the partner's move left alone",
                    key_text(&moved.key),
                    state.roster.objects().len()
                ));
            }
            return;
        };
        state.partner_last_heard.insert(id, moved.position);
        let change = state
            .partner_holds
            .partner_reports(self.now, moved.phase, id, moved.held);
        self.log_partner_hold_changes(state, change.as_slice());
        let i_hold_it = state.local_hold.is_some_and(|(held, _)| held == id);
        match partner_move_verdict(self.i_am_host, i_hold_it, moved.held) {
            MoveVerdict::KeepHostsAndAnswer => self.keep_hosts(state, id, moved.key),
            MoveVerdict::KeepOwnPush => {}
            MoveVerdict::Follow => {
                state.moved_ids.insert(id);
                state.glides.set_target(self.now, id, moved.position);
                state.outbox.note_remote_move(self.now, id);
                if state.carry_log.due(self.now, id) {
                    log_line(&partner_moves_text(
                        &moved.key,
                        moved.position,
                        self.position_of(state, id),
                    ));
                }
            }
        }
    }

    fn keep_hosts(&self, state: &mut ObjectState, id: u16, key: WorldObjectKey) {
        let yours = self.position_of(state, id);
        if state.refusal_log.due(self.now, id) {
            log_line(&refused_guest_move_text(&key, yours));
        }
        let answered = state.refusal_answered_ms.get(&id).copied();
        if answered.is_some_and(|at| self.now.saturating_sub(at) < REFUSAL_ANSWER_INTERVAL_MS) {
            return;
        }
        if let Some(position) = yours {
            state.refusal_answered_ms.insert(id, self.now);
            self.send_moved(key, position, true);
        }
    }

    fn partner_broke(&self, state: &mut ObjectState, broken: WorldObjectBroken) {
        if !partner_shares_phase(Some(self.phase), broken.phase) {
            return;
        }
        if !state
            .pending_breaks
            .iter()
            .any(|(key, _)| *key == broken.key)
        {
            state
                .pending_breaks
                .push((broken.key, self.now + PARTNER_BREAK_RETRY_FOR_MS));
        }
    }

    fn copy_partner_breaks(&self, state: &mut ObjectState) {
        let pending = core::mem::take(&mut state.pending_breaks);
        for (key, until) in pending {
            let actor = self
                .id_of_key(state, &key)
                .and_then(|id| state.roster.actor_of(id))
                .filter(|actor| object_is_live(&self.memory, *actor, key.kind_vtable_rva));
            let hittable = actor.is_some_and(|actor| cube_is_hittable(&self.memory, actor));
            match (partner_break_verdict(actor.is_some(), hittable), actor) {
                (BreakVerdict::BreakHere, Some(actor)) => {
                    break_here(actor);
                    let outcome = if self.i_am_host {
                        "the guest's break is taken, broken here too"
                    } else {
                        "broken here too"
                    };
                    log_line(&partner_break_text(&key, outcome));
                }
                _ if self.now >= until => {
                    log_line(&partner_break_text(
                        &key,
                        "no hittable copy here in time, left alone",
                    ));
                }
                _ => state.pending_breaks.push((key, until)),
            }
        }
    }

    fn send_local_breaks(&self, state: &mut ObjectState) {
        let breaks = take_local_breaks();
        if breaks.is_empty() {
            return;
        }
        if breaks
            .iter()
            .any(|actor| state.roster.id_of_actor(*actor).is_none())
        {
            self.scan(state);
        }
        for actor in breaks {
            let key = state
                .roster
                .id_of_actor(actor)
                .and_then(|id| state.roster.key_of(id))
                .filter(|key| {
                    kind_of_vtable_rva(key.kind_vtable_rva)
                        .is_some_and(|kind| kind.sync == ObjectSync::PushAndBreak)
                });
            let Some(key) = key else {
                log_line(&format!(
                    "objects: a cube broke here as actor {actor:#x}, which no listed object is"
                ));
                continue;
            };
            let broken = WorldObjectBroken {
                phase: self.phase,
                key,
            };
            if broken.validate().is_ok() {
                self.net.send_relayed(Message::WorldObjectBroken(broken));
            }
            log_line(&you_broke_text(&key, self.i_am_host));
        }
    }

    fn glide(&mut self, state: &mut ObjectState) {
        let roster = &state.roster;
        let memory = &self.memory;
        let steps = state.glides.advance(self.now, |id| {
            roster
                .actor_of(id)
                .and_then(|actor| read_block_position(memory, actor))
        });
        for step in steps {
            let Some(actor) = state.roster.actor_of(step.id) else {
                continue;
            };
            if write_block_position(&mut self.memory, actor, step.to).is_err()
                && state.refusal_log.due(self.now, step.id)
            {
                log_line(&format!(
                    "objects: could not move actor {actor:#x} to the partner's place"
                ));
            }
        }
    }

    fn resend_resting_places(&self, state: &mut ObjectState) {
        if !self.i_am_host
            || state.last_resend_ms.is_some_and(|last| {
                self.now.saturating_sub(last) < HOST_RESENDS_MOVED_OBJECTS_EVERY_MS
            })
        {
            return;
        }
        state.last_resend_ms = Some(self.now);
        state
            .moved_ids
            .retain(|id| state.roster.actor_of(*id).is_some());
        let mut resting: Vec<u16> = state
            .moved_ids
            .iter()
            .copied()
            .filter(|id| {
                state.local_hold.is_none_or(|(held, _)| held != *id)
                    && !state.partner_holds.holds(*id)
                    && state.glides.target_of(*id).is_none()
                    && state.outbox.tracking() != Some(*id)
            })
            .collect();
        resting.sort_unstable();
        for id in resting {
            if let (Some(key), Some(position)) =
                (state.roster.key_of(id), self.position_of(state, id))
            {
                self.send_moved(key, position, false);
            }
        }
    }

    fn local_hold(&self, state: &mut ObjectState, player: u64) -> Option<(u16, HoldHow)> {
        let Some(target) = read_held_target(&self.memory, player) else {
            state.unlisted_handle_logged = None;
            return None;
        };
        let owner = handle_owner(&self.memory, target.handle);
        let listed = owner.filter(|owner| kind_of_vtable_rva(owner.vtable_rva).is_some());
        let mut id = listed.and_then(|owner| state.roster.id_of_actor(owner.actor));
        if id.is_none() && listed.is_some() {
            self.scan(state);
            id = listed.and_then(|owner| state.roster.id_of_actor(owner.actor));
        }
        if id.is_none() && state.unlisted_handle_logged != Some(target.handle) {
            state.unlisted_handle_logged = Some(target.handle);
            log_line(&unlisted_hold_text(
                target.handle,
                target.how,
                owner.as_ref(),
            ));
        }
        id.map(|id| (id, target.how))
    }

    fn share_hold(&self, state: &mut ObjectState, held: Option<(u16, HoldHow)>) {
        if state.local_hold.map(|(id, _)| id) != held.map(|(id, _)| id) {
            if let Some((old, _)) = state.local_hold
                && let Some(key) = state.roster.key_of(old)
            {
                log_line(&you_let_go_text(&key, self.position_of(state, old)));
            }
            if let Some((new, how)) = held
                && let Some(key) = state.roster.key_of(new)
            {
                state.carry_log.forget(new);
                log_line(&you_took_text(&key, how, self.position_of(state, new)));
            }
        }
        state.local_hold = held;
        let held_id = held.map(|(id, _)| id);
        if let Some(id) = held_id {
            state.moved_ids.insert(id);
        }
        for announcement in state.announcer.observe(self.now, self.phase, held_id) {
            if announcement.phase != self.phase {
                continue;
            }
            if let (Some(key), Some(position)) = (
                state.roster.key_of(announcement.id),
                self.position_of(state, announcement.id),
            ) {
                self.send_moved(key, position, announcement.held);
            }
        }
        let roster = &state.roster;
        let memory = &self.memory;
        let step = state.outbox.observe(self.now, held_id, |id| {
            roster
                .actor_of(id)
                .and_then(|actor| read_block_position(memory, actor))
        });
        if let Some((id, position)) = step
            && let Some(key) = state.roster.key_of(id)
        {
            self.send_moved(key, position, held_id == Some(id));
        }
        if let Some((id, how)) = held
            && let Some(key) = state.roster.key_of(id)
            && state.carry_log.due(self.now, id)
        {
            log_line(&you_move_text(
                &key,
                how,
                self.position_of(state, id),
                state.partner_last_heard.get(&id).copied(),
            ));
        }
    }
}

pub fn tick_world_objects(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        release_partner_state();
        return;
    };
    let view = net.view();
    if !view.partner_in_play() {
        release_partner_state();
        return;
    }
    let game = Replicant::new(InProcessMemory::new(build));
    let Some(phase) = local_phase_index(&game) else {
        return;
    };
    let player = game.playable_actor().unwrap_or(0);
    let mut tick = Tick {
        build,
        memory: InProcessMemory::new(build),
        net,
        phase,
        now: net_clock_ms(),
        i_am_host: view.local_slot == Some(0),
    };
    let mut guard = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    let state = &mut *guard;
    if state
        .last_scan_ms
        .is_none_or(|last| tick.now.saturating_sub(last) >= WORLD_OBJECT_SCAN_INTERVAL_MS)
    {
        tick.scan(state);
    }
    let changes = state.partner_holds.set_local_phase(phase);
    tick.log_partner_hold_changes(state, &changes);
    let changes = state.partner_holds.expire(tick.now);
    tick.log_partner_hold_changes(state, &changes);
    let wanted = |message: &Message| {
        matches!(
            message,
            Message::WorldObjectMoved(_) | Message::WorldObjectBroken(_)
        )
    };
    for message in net.take_relayed(wanted) {
        match message {
            Message::WorldObjectMoved(moved) => tick.partner_moved(state, moved),
            Message::WorldObjectBroken(broken) => tick.partner_broke(state, broken),
            _ => {}
        }
    }
    tick.send_local_breaks(state);
    tick.copy_partner_breaks(state);
    tick.glide(state);
    let held = tick.local_hold(state, player);
    tick.share_hold(state, held);
    tick.resend_resting_places(state);
    let actors: Vec<u64> = state
        .partner_holds
        .held_ids()
        .into_iter()
        .filter_map(|id| state.roster.actor_of(id))
        .collect();
    publish_partner_held_objects(player, actors);
}

fn release_partner_state() {
    let mut guard = STATE.lock().unwrap_or_else(PoisonError::into_inner);
    let state = &mut *guard;
    state.partner_holds.clear();
    state.partner_last_heard.clear();
    state.pending_breaks.clear();
    state.local_hold = None;
    state.last_resend_ms = None;
    drop(guard);
    let _ = take_local_breaks();
    publish_partner_held_objects(0, Vec::new());
}
