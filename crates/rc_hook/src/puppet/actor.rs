use std::ffi::{CStr, CString};

use rc_game::offsets::{
    ACTOR_UNIQUE_ID_OFFSET, CREATION_MATRIX_BYTES, PUPPET_ACTOR_GROUP, PUPPET_FALLBACK_CLASS_ID,
    PUPPET_UNIQUE_ID_BASE,
};
use rc_game::puppet::{BodyInfo, PuppetPose, actor_is_live};
use rc_game::{GameMemory, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
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
        body: &BodyInfo,
    ) -> Result<Self, PuppetError> {
        let model = CString::new(body.pack_name).map_err(|_| PuppetError::NameHasNul)?;
        let unique_id = PUPPET_UNIQUE_ID_BASE.saturating_add_unsigned(id);
        let calls = GameCalls::new(build);
        let actor = calls.create_model_actor_with_class(
            &model,
            &pose.creation_matrix(),
            PUPPET_FALLBACK_CLASS_ID,
        );
        Self::adopt(build, calls, actor, unique_id)
    }

    pub(super) fn spawn_model(
        build: &VerifiedBuild,
        model: &CStr,
        unique_id: i32,
        matrix: &[u8; CREATION_MATRIX_BYTES],
    ) -> Result<Self, PuppetError> {
        let calls = GameCalls::new(build);
        let actor = calls.create_model_actor(model, matrix, PUPPET_FALLBACK_CLASS_ID);
        Self::adopt(build, calls, actor, unique_id)
    }

    pub(super) fn spawn_model_with_class(
        build: &VerifiedBuild,
        model: &CStr,
        (unique_id, class_id): (i32, i32),
        matrix: &[u8; CREATION_MATRIX_BYTES],
    ) -> Result<Self, PuppetError> {
        let calls = GameCalls::new(build);
        let actor = calls.create_model_actor_with_class(model, matrix, class_id);
        Self::adopt(build, calls, actor, unique_id)
    }

    fn adopt(
        build: &VerifiedBuild,
        calls: GameCalls,
        actor: u64,
        unique_id: i32,
    ) -> Result<Self, PuppetError> {
        if actor == 0 {
            return Err(PuppetError::CreateReturnedNull);
        }
        calls.set_group(actor, PUPPET_ACTOR_GROUP);
        let mut memory = InProcessMemory::new(build);
        if let Err(error) = memory.write_i32(actor + ACTOR_UNIQUE_ID_OFFSET, unique_id) {
            // The actor exists already: destroy it, or every failed attempt leaves one behind.
            calls.destroy(actor);
            return Err(error.into());
        }
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

    pub(super) fn set_matrix(
        &self,
        game: &Replicant<InProcessMemory>,
        matrix: &[u8; CREATION_MATRIX_BYTES],
    ) -> Result<(), PuppetError> {
        self.calls.set_transform(game.memory(), self.actor, matrix)
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

    pub(super) fn despawn_detached(self, game: &Replicant<InProcessMemory>) {
        // The model goes first, as the game does before it frees a weapon pack.
        if let Err(error) = self.calls.detach_model(game.memory(), self.actor) {
            log_line(&format!(
                "puppet {:#x}: cannot detach the model ({error})",
                self.actor
            ));
        }
        self.calls.destroy(self.actor);
    }
}
