use std::collections::{BTreeMap, HashSet};

use rc_game::enemies::{EnemySighting, MirroredEnemy};
use rc_game::offsets::{
    ACTOR_STUB_RETURNING_TRUE_FUNCTION_RVA, ATTACK_FLAG_SKIP_DEFENCE, PLAYER_MESSAGE_DIRECT_HIT,
    PLAYER_MESSAGE_ENEMY_KILLED,
};
use rc_proto::{EnemyId, EnemyKilled, Message};

use crate::diagnostics::log_line;
use crate::enemies::combat::CombatState;
use crate::enemies::combat_view::{CombatView, as_remote, publish_view};
use crate::enemies::drop_hook::drop_at_actor;
use crate::enemies::hold_hook::{hold_update_function, release_held, set_held};
use crate::enemies::sync::LocalView;
use crate::enemies::{COPY_KILL_DAMAGE, EnemySyncRole, PENDING_KILL_MAX_AGE_MS, enemy_id_text};
use crate::memory::InProcessMemory;
use crate::puppet::first_actor_address;

pub(super) fn receive(combat: &mut CombatState, now_ms: u64, message: &Message) {
    match message {
        Message::PlayerHit(hit) => combat.pending_player_hits.push(*hit),
        Message::EnemyKilled(killed) => combat.pending_kills.push((*killed, now_ms)),
        Message::EnemyDrop(drop) => combat.pending_drops.push(*drop),
        _ => {}
    }
}

pub(super) fn step(
    combat: &mut CombatState,
    local: &LocalView<'_>,
    now_ms: u64,
    seen: &[(EnemyId, EnemySighting)],
    samples: &BTreeMap<EnemyId, MirroredEnemy>,
    authority: bool,
) {
    let live: HashSet<u64> = seen.iter().map(|(_, sighting)| sighting.actor).collect();
    combat.killed_copies.retain(|actor| live.contains(actor));
    combat.remember(
        seen.iter()
            .map(|(id, sighting)| (*id, sighting.actor, sighting.spawn_serial)),
    );
    let local_player = local.game.playable_actor().unwrap_or(0);
    let puppet = first_actor_address().unwrap_or(0);
    publish_view(CombatView {
        role: Some(EnemySyncRole::Guest),
        phase: local.phase,
        enemies: combat.actor_ids(),
        local_player,
        puppet,
    });
    let mut memory = InProcessMemory::new(local.build);
    let mut held = HashSet::new();
    for (id, sighting) in seen {
        // Only a copy of a host enemy waits for the host's death; one the host lacks lives here.
        if combat.killed_copies.contains(&sighting.actor) || !samples.contains_key(id) {
            continue;
        }
        local
            .calls
            .set_survive_at_one_hp(&mut memory, sighting.actor, true);
        if authority && hold(local, &memory, sighting.actor) {
            held.insert(sighting.actor);
        }
    }
    combat.counts.held = held.len();
    set_held(held);
    apply_player_hits(combat, local, local_player);
    apply_kills(combat, local, &mut memory, now_ms, local_player, puppet);
    apply_drops(combat, local, local_player);
}

fn hold(local: &LocalView<'_>, memory: &InProcessMemory, actor: u64) -> bool {
    let Some(update) = local.calls.main_update_of(memory, actor) else {
        return false;
    };
    let stub = local.build.module_base() + u64::from(ACTOR_STUB_RETURNING_TRUE_FUNCTION_RVA);
    update as u64 != stub && hold_update_function(update)
}

fn apply_player_hits(combat: &mut CombatState, local: &LocalView<'_>, local_player: u64) {
    let hits = std::mem::take(&mut combat.pending_player_hits);
    let memory = local.game.memory();
    for hit in hits {
        if hit.phase != local.phase || local_player == 0 {
            log_line(&format!(
                "combat: host hit by {} on you not applied (phase {} here {}, player {local_player:#x})",
                enemy_id_text(hit.attacker),
                hit.phase,
                local.phase
            ));
            continue;
        }
        let sender = combat
            .actor_of(memory, hit.attacker)
            .unwrap_or(local_player);
        let before = local.game.health().unwrap_or(-1);
        let sent = as_remote(|| {
            local.calls.send_actor_message(
                memory,
                local_player,
                PLAYER_MESSAGE_DIRECT_HIT,
                sender,
                u64::from(hit.hp_damage),
            )
        });
        let after = local.game.health().unwrap_or(-1);
        if sent {
            combat.counts.player_hits_applied += 1;
        }
        log_line(&format!(
            "combat: {} hit you at the host: hp {} sent, flags {:#x}, kind {}, from {}, your hp {before} -> {after}",
            enemy_id_text(hit.attacker),
            hit.hp_damage,
            hit.flags,
            hit.attack_kind,
            if sender == local_player {
                "no local copy"
            } else {
                "its copy"
            }
        ));
    }
}

fn apply_kills(
    combat: &mut CombatState,
    local: &LocalView<'_>,
    memory: &mut InProcessMemory,
    now_ms: u64,
    local_player: u64,
    puppet: u64,
) {
    let pending = std::mem::take(&mut combat.pending_kills);
    for (killed, received_at) in pending {
        if killed.phase != local.phase {
            continue;
        }
        let Some(copy) = combat.actor_of(memory, killed.id) else {
            if now_ms.saturating_sub(received_at) < PENDING_KILL_MAX_AGE_MS {
                combat.pending_kills.push((killed, received_at));
            } else {
                log_line(&format!(
                    "combat: host killed {}, no copy here to kill",
                    enemy_id_text(killed.id)
                ));
            }
            continue;
        };
        if combat.killed_copies.contains(&copy) {
            continue;
        }
        kill_copy(combat, local, memory, killed, copy, local_player, puppet);
    }
}

fn kill_copy(
    combat: &mut CombatState,
    local: &LocalView<'_>,
    memory: &mut InProcessMemory,
    killed: EnemyKilled,
    copy: u64,
    local_player: u64,
    puppet: u64,
) {
    let paid = local_player != 0
        && local.calls.send_actor_message(
            memory,
            local_player,
            PLAYER_MESSAGE_ENEMY_KILLED,
            copy,
            0,
        );
    combat.killed_copies.insert(copy);
    release_held(copy);
    local.calls.set_survive_at_one_hp(memory, copy, false);
    let attacker = if puppet == 0 { copy } else { puppet };
    as_remote(|| {
        local.calls.direct_hit(
            attacker,
            copy,
            [COPY_KILL_DAMAGE, 0, 0],
            ATTACK_FLAG_SKIP_DEFENCE,
        );
    });
    let hp_after = local.calls.hp_of(memory, copy).map_or(-1, |(hp, _)| hp);
    combat.counts.kills_applied += 1;
    log_line(&format!(
        "combat: host killed {} (by {}); your copy hp now {hp_after}, your kill credit {}",
        enemy_id_text(killed.id),
        if killed.by_guest { "you" } else { "the host" },
        if paid { "paid" } else { "not paid (no player)" }
    ));
}

fn apply_drops(combat: &mut CombatState, local: &LocalView<'_>, local_player: u64) {
    let drops = std::mem::take(&mut combat.pending_drops);
    let memory = local.game.memory();
    for drop in drops {
        if drop.phase != local.phase {
            continue;
        }
        let copy = combat.actor_of(memory, drop.id);
        let at = copy.unwrap_or(local_player);
        let placed = drop_at_actor(at, drop.item_id, drop.count);
        if placed {
            combat.counts.drops_applied += 1;
        }
        log_line(&format!(
            "combat: host's {} dropped item {} x{}: {} at {}",
            enemy_id_text(drop.id),
            drop.item_id,
            drop.count,
            if placed {
                "dropped here"
            } else {
                "not dropped"
            },
            if copy.is_some() {
                "your copy"
            } else {
                "your feet"
            }
        ));
    }
}
