use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use rc_game::fix_buttons::TakeOnce;
use rc_game::puppet::{LayerMotion, PuppetPose};
use rc_game::{PhaseEntry, Position, Replicant};
use rc_proto::{MotionState, PlayerState, StateTrack, TrackSample};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, GhostWish, RemotePlayer, decide_ghost};
use crate::puppet::despawn;
use crate::puppet::registry::{drive_ghost, ghost_exists, spawn_ghost};
use crate::puppet::{GHOST_DELAY_MS, GHOST_PEER_TIMEOUT, GHOST_RESPAWN_COOLDOWN};

static RESPAWN_REQUESTED: TakeOnce = TakeOnce::new();
static GHOST_ENABLED: AtomicBool = AtomicBool::new(true);
static STATUS: Mutex<GhostStatus> = Mutex::new(GhostStatus {
    wish: GhostWish::NotConnected,
    spawned: false,
    spawn_error: None,
});
static GHOST: Mutex<GhostState> = Mutex::new(GhostState::new());

struct GhostState {
    track: Option<StateTrack>,
    epoch: Option<Instant>,
    puppet_id: Option<u32>,
    retry_after: Option<Instant>,
    spawn_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhostStatus {
    pub wish: GhostWish,
    pub spawned: bool,
    pub spawn_error: Option<String>,
}

impl GhostState {
    const fn new() -> Self {
        Self {
            track: None,
            epoch: None,
            puppet_id: None,
            retry_after: None,
            spawn_error: None,
        }
    }
}

pub fn ghost_enabled() -> bool {
    GHOST_ENABLED.load(Ordering::Relaxed)
}

pub fn set_ghost_enabled(enabled: bool) {
    GHOST_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn request_ghost_respawn() {
    RESPAWN_REQUESTED.request();
}

pub fn ghost_status() -> GhostStatus {
    STATUS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

pub fn tick_ghost(build: &VerifiedBuild) {
    let mut ghost = GHOST.lock().unwrap_or_else(PoisonError::into_inner);
    if RESPAWN_REQUESTED.take() {
        drop_puppet_for_respawn(build, &mut ghost);
    }
    let (wish, remote) = wanted_remote(build);
    let Some((received_at, state)) = remote.filter(|_| wish == GhostWish::Wanted) else {
        retire(build, &mut ghost);
        publish_status(wish, &ghost);
        return;
    };
    let epoch = *ghost.epoch.get_or_insert(received_at);
    let at_ms = received_at.saturating_duration_since(epoch).as_millis() as u64;
    let track = ghost.track.get_or_insert_with(StateTrack::new);
    track.push(at_ms, state);
    let now_ms = Instant::now().saturating_duration_since(epoch).as_millis() as u64;
    let Some(sample) = track.sample(now_ms, GHOST_DELAY_MS) else {
        return;
    };
    if let Some(id) = ghost.puppet_id
        && !ghost_exists(id)
    {
        ghost.puppet_id = None;
    }
    if ghost.puppet_id.is_none() {
        try_spawn(build, &mut ghost, &sample);
    }
    if let Some(id) = ghost.puppet_id
        && let Err(error) = drive_ghost(
            build,
            id,
            pose_of(&sample),
            sample.motion.map(layer_motion),
            sample.weapon,
            sample.activity,
        )
    {
        let message = error.to_string();
        if ghost.spawn_error.as_deref() != Some(message.as_str()) {
            log_line(&format!("ghost: {message}"));
        }
        ghost.spawn_error = Some(message);
    }
    publish_status(wish, &ghost);
}

fn drop_puppet_for_respawn(build: &VerifiedBuild, ghost: &mut GhostState) {
    ghost.retry_after = None;
    let Some(id) = ghost.puppet_id.take() else {
        log_line("ghost: respawn requested, no puppet to remove");
        return;
    };
    if ghost_exists(id) {
        let _ = despawn(build, Some(id));
    }
    log_line(&format!(
        "ghost: puppet {id} removed on request, it spawns again"
    ));
}

fn try_spawn(build: &VerifiedBuild, ghost: &mut GhostState, sample: &TrackSample) {
    if ghost
        .retry_after
        .is_some_and(|after| Instant::now() < after)
    {
        return;
    }
    match spawn_ghost(build, pose_of(sample)) {
        Ok(id) => {
            log_line(&format!("ghost: spawned puppet {id}"));
            ghost.puppet_id = Some(id);
            ghost.spawn_error = None;
        }
        Err(error) => {
            let message = error.to_string();
            if ghost.spawn_error.as_deref() != Some(message.as_str()) {
                log_line(&format!("ghost: cannot spawn ({message})"));
            }
            ghost.spawn_error = Some(message);
            ghost.retry_after = Some(Instant::now() + GHOST_RESPAWN_COOLDOWN);
        }
    }
}

fn publish_status(wish: GhostWish, ghost: &GhostState) {
    *STATUS.lock().unwrap_or_else(PoisonError::into_inner) = GhostStatus {
        wish,
        spawned: ghost.puppet_id.is_some(),
        spawn_error: ghost.spawn_error.clone(),
    };
}

fn wanted_remote(build: &VerifiedBuild) -> (GhostWish, Option<(Instant, PlayerState)>) {
    let Some(net) = net::handle() else {
        return (GhostWish::NotConnected, None);
    };
    let view = net.view();
    let game = Replicant::new(InProcessMemory::new(build));
    let local_phase = game
        .current_phase()
        .ok()
        .and_then(|name| PhaseEntry::index_of(&name))
        .and_then(|index| u16::try_from(index).ok());
    let remote = view.remote;
    let wish = decide_ghost(
        ghost_enabled(),
        &view.state,
        remote.as_ref().map(RemotePlayer::age),
        GHOST_PEER_TIMEOUT,
        local_phase,
        remote.as_ref().map_or(0, |remote| remote.state.phase),
    );
    (
        wish,
        remote.map(|remote| (remote.received_at, remote.state)),
    )
}

fn retire(build: &VerifiedBuild, ghost: &mut GhostState) {
    ghost.track = None;
    ghost.epoch = None;
    ghost.retry_after = None;
    ghost.spawn_error = None;
    if let Some(id) = ghost.puppet_id.take() {
        if ghost_exists(id) {
            let _ = despawn(build, Some(id));
        }
        log_line("ghost: peer left the phase or the link, puppet removed");
    }
}

fn pose_of(sample: &TrackSample) -> PuppetPose {
    let [x, y, z] = sample.position;
    PuppetPose {
        position: Position { x, y, z },
        yaw: sample.yaw_radians,
    }
}

fn layer_motion(motion: MotionState) -> LayerMotion {
    LayerMotion {
        index: i32::from(motion.index),
        frame: motion.frame,
        speed: motion.speed,
        looping: motion.looping,
    }
}
