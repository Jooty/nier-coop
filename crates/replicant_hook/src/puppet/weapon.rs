use std::ffi::CString;

use replicant_game::offsets::{
    ACTOR_MODEL_PACK_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET,
    PUPPET_UNIQUE_ID_BASE, WEAPON_IN_HAND_ATTACH_ID, WEAPON_ON_BACK_ATTACH_ID,
};
use replicant_game::puppet::{
    Affine, AttachPoint, RespawnGate, read_attach_point, read_weapon_grip,
};
use replicant_game::{GameMemory, Replicant};
use replicant_proto::WeaponState;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::puppet::actor::Puppet;
use crate::puppet::error::PuppetError;
use crate::puppet::game_calls::GameCalls;
use crate::puppet::pack_guard::{ensure_pack_guard, guard, has_free_slot, still_guarded, unguard};

const WEAPON_UNIQUE_ID_OFFSET: u32 = 0x8000;

#[derive(Debug)]
struct ShownWeapon {
    look: (u8, u8),
    name: CString,
    actor: Puppet,
}

#[derive(Debug, Default)]
pub(super) struct PuppetWeapon {
    shown: Option<ShownWeapon>,
    names: Option<((u8, u8), Option<CString>)>,
    requested: Option<CString>,
    respawn_gate: RespawnGate,
    attach_points: [Option<(CString, AttachPoint)>; 2],
    last_note: Option<String>,
}

impl PuppetWeapon {
    pub(super) fn drive(
        &mut self,
        build: &VerifiedBuild,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        puppet_id: u32,
        weapon: Option<WeaponState>,
    ) {
        self.forget_if_gone(game);
        let Some(weapon) = weapon.filter(|weapon| weapon.visible) else {
            self.despawn(game, true);
            return;
        };
        let look = (weapon.id, weapon.level);
        if self.shown.as_ref().is_some_and(|shown| shown.look != look) {
            self.despawn(game, true);
        }
        let calls = GameCalls::new(build);
        if self.shown.is_none() {
            self.try_show(build, &calls, game, puppet, puppet_id, look);
        }
        if let Err(error) = self.place(&calls, game, puppet, weapon) {
            self.note(format!("puppet weapon: cannot place ({error})"));
        }
    }

    pub(super) fn despawn(&mut self, game: &Replicant<InProcessMemory>, unchecked_is_live: bool) {
        let Some(shown) = self.shown.take() else {
            return;
        };
        if !still_guarded(shown.actor.actor()) {
            return;
        }
        unguard(shown.actor.actor());
        let live = if shown.actor.liveness_is_trusted() {
            shown.actor.is_live(game)
        } else {
            unchecked_is_live
        };
        if live {
            shown.actor.despawn_detached(game);
        }
        log_line(&format!("puppet weapon: {:?} removed", shown.name));
    }

    fn forget_if_gone(&mut self, game: &Replicant<InProcessMemory>) {
        let Some(shown) = &self.shown else {
            return;
        };
        let released = !still_guarded(shown.actor.actor());
        let vanished = shown.actor.liveness_is_trusted() && !shown.actor.is_live(game);
        if released || vanished {
            log_line(&format!(
                "puppet weapon: {:?} is gone ({}), will show it again when its pack is back",
                shown.name,
                if released {
                    "pack released"
                } else {
                    "actor removed"
                }
            ));
            if vanished {
                unguard(shown.actor.actor());
            }
            self.shown = None;
            self.requested = None;
        }
    }

    fn try_show(
        &mut self,
        build: &VerifiedBuild,
        calls: &GameCalls,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        puppet_id: u32,
        look: (u8, u8),
    ) {
        let Some(name) = self.pack_name(calls, look) else {
            self.note(format!("puppet weapon: no model pack for weapon {look:?}"));
            return;
        };
        if !ensure_pack_guard(build) {
            self.note("puppet weapon: release guard missing, weapon stays hidden".to_owned());
            return;
        }
        let pack = calls.find_model_pack(&name);
        if pack == 0 {
            if self.requested.as_ref() != Some(&name) {
                let slot = calls.request_model_pack(&name);
                log_line(&format!(
                    "puppet weapon: loading {name:?} for weapon {look:?} (slot {slot:#x})"
                ));
                self.requested = Some(name);
            }
            return;
        }
        if !self.respawn_gate.may_spawn(true, net_clock_ms()) {
            return;
        }
        if !has_free_slot() {
            self.note("puppet weapon: no free pack guard slot, weapon stays hidden".to_owned());
            return;
        }
        let unique_id = PUPPET_UNIQUE_ID_BASE
            .saturating_add_unsigned(WEAPON_UNIQUE_ID_OFFSET)
            .saturating_add_unsigned(puppet_id);
        let matrix = match game.memory().read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(
            puppet.actor() + ACTOR_TRANSFORM_BLOCK_OFFSET,
        ) {
            Ok(block) => Affine::from_block(&block).to_matrix_bytes(),
            Err(error) => {
                self.note(format!(
                    "puppet weapon: puppet transform unreadable ({error})"
                ));
                return;
            }
        };
        let actor = match Puppet::spawn_model(build, &name, unique_id, &matrix) {
            Ok(actor) => actor,
            Err(error) => {
                self.note(format!("puppet weapon: cannot spawn {name:?} ({error})"));
                return;
            }
        };
        let worn = game
            .memory()
            .read_u64(actor.actor() + ACTOR_MODEL_PACK_OFFSET)
            .unwrap_or(0);
        guard(actor.actor(), if worn == 0 { pack } else { worn });
        log_line(&format!(
            "puppet weapon: showing {name:?} (weapon {}, level {}) as actor {:#x}, pack {pack:#x}, worn {worn:#x}",
            look.0,
            look.1,
            actor.actor()
        ));
        self.shown = Some(ShownWeapon { look, name, actor });
        self.last_note = None;
    }

    fn pack_name(&mut self, calls: &GameCalls, look: (u8, u8)) -> Option<CString> {
        if let Some((cached, name)) = &self.names
            && *cached == look
        {
            return name.clone();
        }
        let name = calls.weapon_pack_name(look.0, look.1);
        self.names = Some((look, name.clone()));
        name
    }

    fn place(
        &mut self,
        calls: &GameCalls,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        weapon: WeaponState,
    ) -> Result<(), PuppetError> {
        let Some(shown) = &self.shown else {
            return Ok(());
        };
        let (attach_id, slot) = if weapon.in_hand {
            (WEAPON_IN_HAND_ATTACH_ID, 0)
        } else {
            (WEAPON_ON_BACK_ATTACH_ID, 1)
        };
        if self.attach_points[slot].is_none() {
            let player = game.playable_actor()?;
            let Some(point) = read_attach_point(game.memory(), player, attach_id)? else {
                return Err(PuppetError::UnknownAttachPoint(attach_id));
            };
            let bone = CString::new(point.bone.as_str()).map_err(|_| PuppetError::NameHasNul)?;
            log_line(&format!(
                "puppet weapon: attach point {attach_id} is bone {:?}, offset {:?}, turn {:?}, root turn {}, target {}",
                point.bone,
                point.translation,
                point.rotation_degrees,
                point.keeps_root_rotation,
                point.target
            ));
            self.attach_points[slot] = Some((bone, point));
        }
        let Some((bone_name, point)) = &self.attach_points[slot] else {
            return Ok(());
        };
        let bone = calls.bone_matrix(game.memory(), puppet.actor(), bone_name)?;
        let root = game.memory().read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(
            puppet.actor() + ACTOR_TRANSFORM_BLOCK_OFFSET,
        )?;
        let grip = read_weapon_grip(game.memory(), weapon.id, weapon.in_hand)?.unwrap_or([0.0; 3]);
        let world = point
            .frame(
                &Affine::from_block(&block_of(&bone)),
                &Affine::from_block(&root),
            )
            .then(&Affine::translation(grip));
        shown.actor.set_matrix(game, &world.to_matrix_bytes())
    }

    fn note(&mut self, message: String) {
        if self.last_note.as_deref() != Some(message.as_str()) {
            log_line(&message);
            self.last_note = Some(message);
        }
    }
}

fn block_of(matrix: &[u8; 64]) -> [u8; ACTOR_TRANSFORM_BLOCK_BYTES] {
    let mut block = [0_u8; ACTOR_TRANSFORM_BLOCK_BYTES];
    block.copy_from_slice(&matrix[..ACTOR_TRANSFORM_BLOCK_BYTES]);
    block
}
