use std::collections::HashMap;

use replicant_proto::{COMPANION_ENEMY_GROUP, COMPANION_SLOT_COUNT, EnemyId};

use crate::enemies::EnemySighting;
use crate::memory::GameMemory;
use crate::npcs::read_enemy_body_layer;
use crate::offsets::{
    ACTOR_CLASS_ID_OFFSET, ACTOR_LIFE_STATE_OFFSET, ACTOR_SPAWN_SERIAL_OFFSET,
    ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET, PARTNER_ACTIVE_FLAG_OFFSET,
    PARTNER_KAINE_ACTOR_RVA, PARTNER_SCRIPT_MODE_OFFSET, PARTNER_THIRD_MEMBER_ACTOR_RVA,
    PARTNER_VARIANT_POINTER_RVA,
};
use crate::puppet::{PuppetPose, actor_is_live};

const COMPANION_NAMES: [&str; COMPANION_SLOT_COUNT as usize] = [
    "unique actor -2",
    "the third member",
    "the variant companion",
];

/// One of the game's own companions that is active on this machine this tick. The game's
/// scripts make it active and take it away; the sync never does either.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompanionReading {
    pub id: EnemyId,
    pub sighting: EnemySighting,
    /// A script on this machine drives it (`_StartUniqueActorScriptMode`).
    pub script_mode: bool,
}

/// Who moves a companion on the guest this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionControl {
    Host,
    OwnLogic(CompanionOwnLogicCause),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionOwnLogicCause {
    HostDoesNotSendIt,
    SnapOff,
    EventScene,
    ScriptMode,
    HostMotionUnknown,
}

/// `host_motion_known` is `None` when the host sends no state for this companion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanionGuestInputs {
    pub host_motion_known: Option<bool>,
    pub snap_enabled: bool,
    pub in_event_scene: bool,
    pub script_mode: bool,
}

/// Remembers who moved each companion last tick, so only a change is logged.
#[derive(Debug, Default)]
pub struct CompanionControlLog {
    last: HashMap<EnemyId, CompanionControl>,
}

impl CompanionControlLog {
    pub fn reset(&mut self) {
        self.last.clear();
    }

    /// The control of every companion active this tick; returns the ones that changed.
    pub fn observe(
        &mut self,
        controls: &[(EnemyId, CompanionControl)],
    ) -> Vec<(EnemyId, CompanionControl)> {
        let changed = controls
            .iter()
            .filter(|(id, control)| self.last.get(id) != Some(control))
            .copied()
            .collect();
        self.last = controls.iter().copied().collect();
        changed
    }
}

/// The guest's hold rule. A scene or a script on this machine always wins, so nothing the game
/// itself poses or waits on is ever held; then the host's state, when it is complete.
pub fn companion_control(inputs: CompanionGuestInputs) -> CompanionControl {
    let own = CompanionControl::OwnLogic;
    if inputs.in_event_scene {
        return own(CompanionOwnLogicCause::EventScene);
    }
    if inputs.script_mode {
        return own(CompanionOwnLogicCause::ScriptMode);
    }
    match inputs.host_motion_known {
        None => own(CompanionOwnLogicCause::HostDoesNotSendIt),
        Some(_) if !inputs.snap_enabled => own(CompanionOwnLogicCause::SnapOff),
        Some(false) => own(CompanionOwnLogicCause::HostMotionUnknown),
        Some(true) => CompanionControl::Host,
    }
}

/// The host sends no companion while one of its event scenes plays or has just ended: the
/// scene's poses use motions the guest's copy may not hold.
pub fn host_sends_companion(in_event_scene: bool, scene_settling: bool) -> bool {
    !in_event_scene && !scene_settling
}

/// A guest's own hit on a companion goes to the host only when the guest's game took it (the
/// companion's HP fell), so a swing through an ally never hurts the host's.
pub fn companion_hit_counts(hp_before: Option<i32>, hp_after: Option<i32>) -> bool {
    matches!((hp_before, hp_after), (Some(before), Some(after)) if after < before)
}

pub fn companion_name(id: EnemyId) -> &'static str {
    COMPANION_NAMES
        .get(usize::from(id.ordinal))
        .copied()
        .filter(|_| id.group == COMPANION_ENEMY_GROUP)
        .unwrap_or("an unknown companion")
}

pub fn companion_control_text(id: EnemyId, control: CompanionControl) -> String {
    let name = companion_name(id);
    match control {
        CompanionControl::Host => {
            format!("{name} is held here and follows the host's (place, motion and life)")
        }
        CompanionControl::OwnLogic(cause) => {
            let cause = match cause {
                CompanionOwnLogicCause::HostDoesNotSendIt => {
                    "the host sends none (not active there, or in a scene there)"
                }
                CompanionOwnLogicCause::SnapOff => "enemy snapping is off",
                CompanionOwnLogicCause::EventScene => "an event scene plays here",
                CompanionOwnLogicCause::ScriptMode => "a script drives it here",
                CompanionOwnLogicCause::HostMotionUnknown => {
                    "the host's motion for it is not readable"
                }
            };
            format!("{name} is left to this game: {cause}")
        }
    }
}

/// Every companion that is active on this machine, with its fixed network id. `hp_of(actor)`
/// reads `(hp, max_hp)` through the parameter object; a companion it cannot read is sent with
/// no life, which the guest leaves alone.
pub fn read_companions<M: GameMemory>(
    memory: &M,
    mut hp_of: impl FnMut(u64) -> Option<(i32, i32)>,
) -> Vec<CompanionReading> {
    let base = memory.module_base();
    let variant = memory
        .read_u64(base + u64::from(PARTNER_VARIANT_POINTER_RVA))
        .unwrap_or(0);
    let slots = [
        base + u64::from(PARTNER_KAINE_ACTOR_RVA),
        base + u64::from(PARTNER_THIRD_MEMBER_ACTOR_RVA),
        variant,
    ];
    (0_u16..)
        .zip(slots)
        .filter(|(_, actor)| *actor != 0)
        .filter_map(|(slot, actor)| {
            read_companion(memory, EnemyId::companion(slot), actor, &mut hp_of)
        })
        .collect()
}

fn read_companion<M: GameMemory>(
    memory: &M,
    id: EnemyId,
    actor: u64,
    hp_of: &mut impl FnMut(u64) -> Option<(i32, i32)>,
) -> Option<CompanionReading> {
    if !actor_is_live(memory, actor).ok()? {
        return None;
    }
    // A partner object exists for the whole run; the game's "is active" (vslot 0x120) is this byte.
    if memory.read_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET).ok()? == 0 {
        return None;
    }
    if memory.read_i32(actor + ACTOR_LIFE_STATE_OFFSET).ok()? > 0 {
        return None;
    }
    let block = memory
        .read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
        .ok()?;
    let pose = PuppetPose::from_transform_block(&block);
    let (hp, max_hp) = hp_of(actor).unwrap_or((0, 0));
    Some(CompanionReading {
        id,
        sighting: EnemySighting {
            actor,
            spawn_serial: memory.read_i32(actor + ACTOR_SPAWN_SERIAL_OFFSET).ok()?,
            group: id.group,
            class_id: memory.read_i32(actor + ACTOR_CLASS_ID_OFFSET).unwrap_or(-1),
            position: [pose.position.x, pose.position.y, pose.position.z],
            yaw_radians: pose.yaw,
            motion: read_enemy_body_layer(memory, actor)
                .ok()
                .flatten()
                .map(|layer| layer.motion),
            hp,
            max_hp,
        },
        script_mode: memory.read_i32(actor + PARTNER_SCRIPT_MODE_OFFSET).ok()? != 0,
    })
}
