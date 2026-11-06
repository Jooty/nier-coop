use rc_game::Replicant;
use rc_game::offsets::BODY_MOTION_LAYER;
use rc_game::puppet::{
    LayerMotion, MIRROR_BLEND_FRAMES, MIRROR_FRAME_DRIFT_LIMIT, PuppetPose, motion_name,
    read_layer_motion,
};

use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::puppet::actor::Puppet;
use crate::puppet::error::PuppetError;

#[derive(Debug)]
pub(super) struct RemoteDriver {
    cached_name: Option<(i32, Option<String>)>,
    resolved: Option<(i32, i32)>,
}

impl RemoteDriver {
    pub(super) fn new() -> Self {
        Self {
            cached_name: None,
            resolved: None,
        }
    }

    pub(super) fn drive_remote(
        &mut self,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        pose: PuppetPose,
        motion: Option<LayerMotion>,
    ) -> Result<(), PuppetError> {
        let motion = match (motion, game.playable_actor()) {
            (Some(motion), Ok(player)) => {
                let name = self.name_of(game, player, motion.index);
                Some((motion, name))
            }
            (Some(motion), Err(_)) => Some((motion, None)),
            (None, _) => None,
        };
        self.drive(game, puppet, pose, motion)
    }

    fn drive(
        &mut self,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        pose: PuppetPose,
        motion: Option<(LayerMotion, Option<String>)>,
    ) -> Result<(), PuppetError> {
        puppet.set_pose(game, pose)?;
        if let Some((target, name)) = motion {
            self.follow_motion(game, puppet, &target, name.as_deref())?;
        }
        Ok(())
    }

    fn name_of(
        &mut self,
        game: &Replicant<InProcessMemory>,
        player: u64,
        index: i32,
    ) -> Option<String> {
        if let Some((cached_index, name)) = &self.cached_name
            && *cached_index == index
        {
            return name.clone();
        }
        let name = motion_name(game.memory(), player, index).ok().flatten();
        self.cached_name = Some((index, name.clone()));
        name
    }

    fn follow_motion(
        &mut self,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        target: &LayerMotion,
        name: Option<&str>,
    ) -> Result<(), PuppetError> {
        let index = self.puppet_index(puppet, target.index, name)?;
        let current = read_layer_motion(game.memory(), puppet.actor(), BODY_MOTION_LAYER)?;
        let blend = match current {
            Some(current) if current.index == index => {
                if (current.frame - target.frame).abs() <= MIRROR_FRAME_DRIFT_LIMIT {
                    return Ok(());
                }
                0.0
            }
            _ => MIRROR_BLEND_FRAMES,
        };
        puppet.play_motion(
            BODY_MOTION_LAYER,
            index,
            blend,
            target.looping,
            target.speed,
            target.frame,
        );
        Ok(())
    }

    fn puppet_index(
        &mut self,
        puppet: &Puppet,
        player_index: i32,
        name: Option<&str>,
    ) -> Result<i32, PuppetError> {
        if let Some((from, to)) = self.resolved
            && from == player_index
        {
            return Ok(to);
        }
        let by_name = match name {
            Some(name) => puppet.motion_index_by_name(name)?,
            None => -1,
        };
        let index = if by_name >= 0 { by_name } else { player_index };
        if index != player_index || name.is_none() {
            log_line(&format!(
                "puppet: motion {name:?} player index {player_index} resolves to {index} on the puppet"
            ));
        }
        self.resolved = Some((player_index, index));
        Ok(index)
    }
}
