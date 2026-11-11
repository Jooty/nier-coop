use std::collections::{BTreeMap, BTreeSet};

use replicant_game::enemies::{
    CompanionControl, CompanionControlLog, CompanionGuestInputs, CompanionReading, MirroredEnemy,
    companion_control, companion_control_text, companion_name, host_sends_companion,
    read_companions,
};
use replicant_proto::EnemyId;

use crate::diagnostics::log_line;
use crate::enemies::combat::{CombatState, LocalEnemy};
use crate::enemies::combat_view::add_shared_actors;
use crate::enemies::hold_hook::hold_also;
use crate::enemies::sync::LocalView;

/// What the companion share remembers between ticks, on either side, to log only changes.
#[derive(Debug, Default)]
pub(super) struct CompanionShare {
    guest_controls: CompanionControlLog,
    host_sent: BTreeSet<EnemyId>,
}

/// A companion of this game the guest holds this tick, with the host's state to pose it by.
#[derive(Debug, Clone, Copy)]
pub(super) struct DrivenCompanion {
    pub(super) id: EnemyId,
    pub(super) actor: u64,
    pub(super) host: MirroredEnemy,
}

/// The host's active companions to send this tick. They join the shared actors, so a guest's
/// hit on one is applied here like a hit on a shared enemy.
pub(super) fn host_companions(
    share: &mut CompanionShare,
    combat: &mut CombatState,
    local: &LocalView<'_>,
    in_event_scene: bool,
    scene_settling: bool,
) -> Vec<CompanionReading> {
    let companions = if host_sends_companion(in_event_scene, scene_settling) {
        read(local)
    } else {
        Vec::new()
    };
    let sent: BTreeSet<EnemyId> = companions.iter().map(|companion| companion.id).collect();
    for id in sent.difference(&share.host_sent) {
        log_line(&format!(
            "companion (host): {} is sent to the guest, whose own follows it",
            companion_name(*id)
        ));
    }
    for id in share.host_sent.difference(&sent) {
        log_line(&format!(
            "companion (host): {} is no longer sent (not active here, or a scene plays here{})",
            companion_name(*id),
            if scene_settling {
                " and just ended"
            } else {
                ""
            }
        ));
    }
    share.host_sent = sent;
    join_shared_actors(combat, &companions);
    companions
}

/// Decides who moves each of the guest's active companions, holds the ones the host moves and
/// returns them. Call after the enemy step: it rebuilds the held set and the shared actors.
pub(super) fn guest_companions(
    share: &mut CompanionShare,
    combat: &mut CombatState,
    local: &LocalView<'_>,
    hosts: &BTreeMap<EnemyId, MirroredEnemy>,
    snap_enabled: bool,
    in_event_scene: bool,
) -> Vec<DrivenCompanion> {
    let here = read(local);
    let controls: Vec<(EnemyId, CompanionControl)> = here
        .iter()
        .map(|companion| {
            let control = companion_control(CompanionGuestInputs {
                host_motion_known: hosts.get(&companion.id).map(|host| host.motion.is_some()),
                snap_enabled,
                in_event_scene,
                script_mode: companion.script_mode,
            });
            (companion.id, control)
        })
        .collect();
    for (id, control) in share.guest_controls.observe(&controls) {
        log_line(&format!(
            "companion (guest): {}",
            companion_control_text(id, control)
        ));
    }
    let shared: Vec<CompanionReading> = here
        .iter()
        .filter(|companion| hosts.contains_key(&companion.id))
        .copied()
        .collect();
    join_shared_actors(combat, &shared);
    let driven: Vec<DrivenCompanion> = here
        .iter()
        .zip(&controls)
        .filter(|(_, (_, control))| *control == CompanionControl::Host)
        .filter_map(|(companion, _)| {
            Some(DrivenCompanion {
                id: companion.id,
                actor: companion.sighting.actor,
                host: *hosts.get(&companion.id)?,
            })
        })
        .collect();
    hold_also(driven.iter().map(|companion| companion.actor));
    driven
}

fn read(local: &LocalView<'_>) -> Vec<CompanionReading> {
    let memory = local.game.memory();
    let calls = local.calls;
    read_companions(memory, |actor| calls.hp_of(memory, actor))
}

fn join_shared_actors(combat: &mut CombatState, companions: &[CompanionReading]) {
    for companion in companions {
        combat.actors.insert(
            companion.id,
            LocalEnemy {
                actor: companion.sighting.actor,
                serial: companion.sighting.spawn_serial,
            },
        );
    }
    add_shared_actors(
        companions
            .iter()
            .map(|companion| (companion.sighting.actor, companion.id)),
    );
}
