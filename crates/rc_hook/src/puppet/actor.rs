use std::ffi::CString;

use rc_game::offsets::{
    ACTOR_UNIQUE_ID_OFFSET, PUPPET_ACTOR_GROUP, PUPPET_FALLBACK_CLASS_ID, PUPPET_MODEL_PACK_NAME,
    PUPPET_UNIQUE_ID_BASE,
};
use rc_game::puppet::{PuppetPose, actor_is_live};
use rc_game::{GameMemory, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;
use crate::puppet::error::PuppetError;
use crate::puppet::game_calls::GameCalls;

#[derive(Debug)]
pub(super) struct Puppet {
    actor: u64,
    calls: GameCalls,
    liveness_is_trusted: bool,
}

impl Puppet {
    pub(super) fn spawn(
        build: &VerifiedBuild,
        id: u32,
        pose: PuppetPose,
    ) -> Result<Self, PuppetError> {
        let calls = GameCalls::new(build);
        let model = CString::new(PUPPET_MODEL_PACK_NAME).map_err(|_| PuppetError::NameHasNul)?;
        let actor =
            calls.create_model_actor(&model, &pose.creation_matrix(), PUPPET_FALLBACK_CLASS_ID);
        if actor == 0 {
            return Err(PuppetError::CreateReturnedNull);
        }
        calls.set_group(actor, PUPPET_ACTOR_GROUP);
        let mut memory = InProcessMemory::new(build);
        let unique_id = PUPPET_UNIQUE_ID_BASE.saturating_add_unsigned(id);
        memory.write_i32(actor + ACTOR_UNIQUE_ID_OFFSET, unique_id)?;
        let liveness_is_trusted = actor_is_live(&memory, actor).unwrap_or(false);
        Ok(Self {
            actor,
            calls,
            liveness_is_trusted,
        })
    }

    pub(super) fn actor(&self) -> u64 {
        self.actor
    }

    pub(super) fn liveness_is_trusted(&self) -> bool {
        self.liveness_is_trusted
    }

    pub(super) fn is_live(&self, game: &Replicant<InProcessMemory>) -> bool {
        actor_is_live(game.memory(), self.actor).unwrap_or(false)
    }

    pub(super) fn set_pose(
        &self,
        game: &Replicant<InProcessMemory>,
        pose: PuppetPose,
    ) -> Result<(), PuppetError> {
        self.calls
            .set_transform(game.memory(), self.actor, &pose.creation_matrix())
    }

    pub(super) fn motion_index_by_name(&self, name: &str) -> Result<i32, PuppetError> {
        let name = CString::new(name).map_err(|_| PuppetError::NameHasNul)?;
        Ok(self.calls.motion_index_by_name(self.actor, &name))
    }

    pub(super) fn play_motion(
        &self,
        layer: u32,
        index: i32,
        blend: f32,
        looping: bool,
        speed: f32,
        start_frame: f32,
    ) {
        self.calls
            .play_motion(self.actor, layer, index, blend, looping, speed, start_frame);
    }

    pub(super) fn despawn(self) {
        self.calls.destroy(self.actor);
    }
}
