use std::time::Instant;

use rc_game::Replicant;
use rc_game::offsets::BODY_MOTION_LAYER;
use rc_game::puppet::{
    DelayLine, LayerMotion, MIRROR_BLEND_FRAMES, MIRROR_FRAME_DRIFT_LIMIT,
    MIRROR_SIDE_OFFSET_UNITS, PuppetPose, motion_name, read_layer_motion, read_pose,
};

use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::puppet::actor::Puppet;
use crate::puppet::delay_setting::mirror_delay_ms;
use crate::puppet::error::PuppetError;

#[derive(Debug, Clone)]
struct MirrorSample {
    pose: PuppetPose,
    motion: Option<(LayerMotion, Option<String>)>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Observed {
    pub(super) pose: Option<PuppetPose>,
    pub(super) motion_index: Option<i32>,
    pub(super) motion_name: Option<String>,
    pub(super) frame: Option<f32>,
}

#[derive(Debug)]
pub(super) struct MirrorDriver {
    line: DelayLine<MirrorSample>,
    started: Instant,
    cached_name: Option<(i32, Option<String>)>,
    resolved: Option<(i32, i32)>,
}

impl MirrorDriver {
    pub(super) fn new() -> Self {
        Self {
            line: DelayLine::default(),
            started: Instant::now(),
            cached_name: None,
            resolved: None,
        }
    }

    pub(super) fn tick(
        &mut self,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
    ) -> Result<Observed, PuppetError> {
        let memory = game.memory();
        let player = game.playable_actor()?;
        let pose = read_pose(memory, player)?;
        let motion = match read_layer_motion(memory, player, BODY_MOTION_LAYER)? {
            Some(motion) => {
                let name = self.name_of(game, player, motion.index);
                Some((motion, name))
            }
            None => None,
        };
        let now_ms = self.started.elapsed().as_millis() as u64;
        self.line.push(now_ms, MirrorSample { pose, motion });
        let Some(sample) = self
            .line
            .sample_delayed(now_ms, u64::from(mirror_delay_ms()))
            .cloned()
        else {
            return Ok(Observed::default());
        };
        let target_pose = sample.pose.shifted_sideways(MIRROR_SIDE_OFFSET_UNITS);
        self.drive(game, puppet, target_pose, sample.motion)
    }

    pub(super) fn drive_remote(
        &mut self,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        pose: PuppetPose,
        motion: Option<LayerMotion>,
    ) -> Result<Observed, PuppetError> {
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
    ) -> Result<Observed, PuppetError> {
        puppet.set_pose(game, pose)?;
        let mut observed = Observed {
            pose: Some(pose),
            ..Observed::default()
        };
        if let Some((target, name)) = motion {
            self.follow_motion(game, puppet, &target, name.as_deref())?;
            observed.motion_index = Some(target.index);
            observed.motion_name = name;
            observed.frame = Some(target.frame);
        }
        Ok(observed)
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
