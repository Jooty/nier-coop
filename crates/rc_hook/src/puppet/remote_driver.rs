use std::time::{Duration, Instant};

use rc_game::Replicant;
use rc_game::offsets::BODY_MOTION_LAYER;
use rc_game::puppet::{
    IDLE_MOTION_NAME, LayerMotion, MOTION_BLEND_SECONDS, MotionStep, PuppetLayer, PuppetPose,
    follow_step, frame_to_seek, motion_name, read_puppet_layer, seek_layer_frame,
};
use rc_proto::{MOTION_FRAMES_PER_MS, PartnerActivity};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::puppet::actor::Puppet;
use crate::puppet::error::PuppetError;

const MOTION_LOG_WINDOW: Duration = Duration::from_secs(5);
const MOTION_LINES_PER_WINDOW: u32 = 40;

#[derive(Debug)]
pub(super) struct RemoteDriver {
    cached_name: Option<(i32, Option<String>)>,
    resolved: Option<(i32, Option<i32>)>,
    idle_index: Option<Option<i32>>,
    idle_since: Option<Instant>,
    last_started: Option<i32>,
    last_seen: Option<(i32, f32)>,
    layer_read_failure_logged: bool,
    missing_motion_logged: bool,
    motion_log: MotionLog,
}

#[derive(Debug)]
struct MotionLog {
    window_start: Instant,
    changes: u32,
    unlogged: u32,
    seeks: u32,
    ticks: u32,
    own_advance: f32,
    own_advance_ticks: u32,
}

impl RemoteDriver {
    pub(super) fn new() -> Self {
        Self {
            cached_name: None,
            resolved: None,
            idle_index: None,
            idle_since: None,
            last_started: None,
            last_seen: None,
            layer_read_failure_logged: false,
            missing_motion_logged: false,
            motion_log: MotionLog::new(),
        }
    }

    pub(super) fn drive_remote(
        &mut self,
        build: &VerifiedBuild,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        pose: PuppetPose,
        motion: Option<LayerMotion>,
        activity: PartnerActivity,
    ) -> Result<(), PuppetError> {
        puppet.set_pose(game, pose)?;
        self.motion_log.roll(puppet.actor());
        if activity.world_paused() {
            return self.hold_idle(build, game, puppet, activity);
        }
        if self.idle_since.take().is_some() {
            log_line(&format!(
                "puppet {:#x}: partner is back in play, following their motion again",
                puppet.actor()
            ));
        }
        let Some(motion) = motion else {
            if !self.missing_motion_logged {
                self.missing_motion_logged = true;
                log_line("puppet: the remote state carries no body motion yet");
            }
            return Ok(());
        };
        let name = match game.playable_actor() {
            Ok(player) => self.name_of(game, player, motion.index),
            Err(_) => None,
        };
        self.follow_motion(build, game, puppet, &motion, name.as_deref())
    }

    fn hold_idle(
        &mut self,
        build: &VerifiedBuild,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        activity: PartnerActivity,
    ) -> Result<(), PuppetError> {
        let idle = match self.idle_index {
            Some(index) => index,
            None => {
                let by_name = puppet.motion_index_by_name(IDLE_MOTION_NAME)?;
                let index = (by_name >= 0).then_some(by_name);
                self.idle_index = Some(index);
                index
            }
        };
        let Some(idle) = idle else {
            return Ok(());
        };
        let since = match self.idle_since {
            Some(since) => since,
            None => {
                let now = Instant::now();
                self.idle_since = Some(now);
                log_line(&format!(
                    "puppet {:#x}: partner is {}, holding the idle loop",
                    puppet.actor(),
                    activity.label().unwrap_or("paused")
                ));
                now
            }
        };
        if self.last_started != Some(idle) {
            puppet.play_motion(
                BODY_MOTION_LAYER,
                idle,
                MOTION_BLEND_SECONDS,
                true,
                1.0,
                0.0,
            );
            self.last_started = Some(idle);
            return Ok(());
        }
        let frame = since.elapsed().as_millis() as f32 * MOTION_FRAMES_PER_MS;
        let layer = self.puppet_layer(game, puppet);
        self.seek(build, layer, idle, frame);
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
        build: &VerifiedBuild,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
        target: &LayerMotion,
        name: Option<&str>,
    ) -> Result<(), PuppetError> {
        let Some(index) = self.puppet_index(puppet, target.index, name)? else {
            return Ok(());
        };
        let layer = self.puppet_layer(game, puppet);
        let MotionStep::Start { blend } =
            follow_step(layer.map(|layer| layer.motion), self.last_started, index)
        else {
            self.seek(build, layer, index, target.frame);
            return Ok(());
        };
        puppet.play_motion(
            BODY_MOTION_LAYER,
            index,
            blend,
            target.looping,
            target.speed,
            target.frame,
        );
        if self.last_started.is_none() {
            log_line(&format!(
                "puppet {:#x}: first motion {index} ({}) from frame {:.1}",
                puppet.actor(),
                name.unwrap_or("unnamed"),
                target.frame
            ));
        } else {
            self.motion_log
                .change(puppet.actor(), index, name, target.frame, blend);
        }
        self.last_started = Some(index);
        self.last_seen = None;
        Ok(())
    }

    fn puppet_layer(
        &mut self,
        game: &Replicant<InProcessMemory>,
        puppet: &Puppet,
    ) -> Option<PuppetLayer> {
        match read_puppet_layer(game.memory(), puppet.actor(), BODY_MOTION_LAYER) {
            Ok(layer) => layer,
            Err(error) => {
                if !self.layer_read_failure_logged {
                    self.layer_read_failure_logged = true;
                    log_line(&format!(
                        "puppet {:#x}: body layer unreadable ({error}), following by started motion",
                        puppet.actor()
                    ));
                }
                None
            }
        }
    }

    fn seek(
        &mut self,
        build: &VerifiedBuild,
        layer: Option<PuppetLayer>,
        index: i32,
        target_frame: f32,
    ) {
        // Only the slot that plays our motion is touched; a fading-in start is left alone.
        let Some(layer) = layer.filter(|layer| layer.motion.index == index) else {
            return;
        };
        self.motion_log.ticks += 1;
        if let Some((seen_index, seen_frame)) = self.last_seen
            && seen_index == index
            && layer.motion.frame >= seen_frame
        {
            self.motion_log.own_advance += layer.motion.frame - seen_frame;
            self.motion_log.own_advance_ticks += 1;
        }
        let mut now_at = layer.motion.frame;
        if let Some(frame) = frame_to_seek(
            layer.motion.frame,
            target_frame,
            layer.frame_count,
            layer.motion.looping,
        ) && seek_layer_frame(&mut InProcessMemory::new(build), &layer, frame).is_ok()
        {
            self.motion_log.seeks += 1;
            now_at = frame;
        }
        self.last_seen = Some((index, now_at));
    }

    fn puppet_index(
        &mut self,
        puppet: &Puppet,
        player_index: i32,
        name: Option<&str>,
    ) -> Result<Option<i32>, PuppetError> {
        if let Some((from, to)) = self.resolved
            && from == player_index
        {
            return Ok(to);
        }
        let index = match name {
            Some(name) => {
                let by_name = puppet.motion_index_by_name(name)?;
                // A name the puppet lacks would play a wrong motion by index: keep the last one.
                (by_name >= 0).then_some(by_name)
            }
            None => Some(player_index),
        };
        if index != Some(player_index) || name.is_none() {
            log_line(&format!(
                "puppet: motion {name:?} player index {player_index} resolves to {index:?} on the puppet"
            ));
        }
        self.resolved = Some((player_index, index));
        Ok(index)
    }
}

impl MotionLog {
    fn new() -> Self {
        Self {
            window_start: Instant::now(),
            changes: 0,
            unlogged: 0,
            seeks: 0,
            ticks: 0,
            own_advance: 0.0,
            own_advance_ticks: 0,
        }
    }

    fn change(&mut self, actor: u64, index: i32, name: Option<&str>, frame: f32, blend: f32) {
        self.changes += 1;
        if self.changes > MOTION_LINES_PER_WINDOW {
            self.unlogged += 1;
            return;
        }
        log_line(&format!(
            "puppet {actor:#x}: motion {index} ({}) from frame {frame:.1}, blend {blend:.2} s",
            name.unwrap_or("unnamed")
        ));
    }

    fn roll(&mut self, actor: u64) {
        if self.window_start.elapsed() < MOTION_LOG_WINDOW {
            return;
        }
        if self.ticks > 0 || self.unlogged > 0 {
            let own = if self.own_advance_ticks > 0 {
                self.own_advance / self.own_advance_ticks as f32
            } else {
                0.0
            };
            log_line(&format!(
                "puppet {actor:#x}: last {} s: {} motion changes ({} not logged), {} frame seeks in {} ticks, puppet advances {own:.2} frames per tick by itself",
                MOTION_LOG_WINDOW.as_secs(),
                self.changes,
                self.unlogged,
                self.seeks,
                self.ticks
            ));
        }
        *self = Self::new();
    }
}
